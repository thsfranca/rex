from __future__ import annotations

import asyncio
import hashlib
import logging
import time
from datetime import datetime, timezone

import httpx
import litellm
from fastapi import Request, Response
from fastapi.responses import JSONResponse, StreamingResponse

from app.adapters.base import ClientAdapter
from app.adapters.default import DefaultAdapter
from app.config import ModelConfig
from app.enrichment.context import EnrichmentContext
from app.enrichment.pipeline import EnrichmentPipeline
from app.logging.models import DecisionRecord
from app.logging.repository import DecisionRepository
from app.proxy.anthropic import (
    anthropic_to_openai,
    extract_anthropic_api_key,
    openai_response_to_anthropic,
    stream_anthropic_response,
)
from app.proxy.message_sanitizer import sanitize_messages
from app.proxy.streaming import stream_completion
from app.router.engine import RoutingEngine
from app.utils import extract_last_user_text

_DEFAULT_ADAPTER = DefaultAdapter()

logger = logging.getLogger(__name__)

LITELLM_PASSTHROUGH_PARAMS = frozenset(
    {
        "messages",
        "temperature",
        "max_tokens",
        "top_p",
        "n",
        "stop",
        "presence_penalty",
        "frequency_penalty",
        "logit_bias",
        "user",
        "functions",
        "function_call",
        "tools",
        "tool_choice",
        "response_format",
        "seed",
    }
)


def _extract_bearer_token(authorization: str | None) -> str | None:
    if not authorization:
        return None
    if authorization.startswith("Bearer "):
        return authorization[7:]
    return None


def _build_litellm_params(
    body: dict, model_config: ModelConfig, request_api_key: str | None = None
) -> dict:
    params = {"model": model_config.name}
    if model_config.api_key:
        params["api_key"] = model_config.api_key
    elif request_api_key:
        params["api_key"] = request_api_key
    if model_config.api_base:
        params["api_base"] = model_config.api_base

    for key in LITELLM_PASSTHROUGH_PARAMS:
        if key in body:
            params[key] = body[key]

    return params


async def _call_with_fallback(
    engine: RoutingEngine,
    primary: ModelConfig,
    body: dict,
    stream: bool,
    request_api_key: str | None = None,
):
    models_to_try = [primary] + engine.fallback_order(primary)
    last_error = None

    for model in models_to_try:
        try:
            params = _build_litellm_params(body, model, request_api_key)
            if stream:
                params["stream"] = True
            response = await litellm.acompletion(**params)
            return response, model
        except Exception as e:
            logger.warning("Model %s failed: %s", model.name, e)
            last_error = e

    raise last_error


def _hash_prompt(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


async def _log_decision(
    repository: DecisionRepository,
    decision,
    used_model: ModelConfig,
    response_time_ms: int,
    response,
    prompt_hash: str,
    embedding_bytes: bytes | None,
) -> None:
    try:
        input_tokens = None
        output_tokens = None
        cost = None

        if hasattr(response, "usage") and response.usage is not None:
            input_tokens = getattr(response.usage, "prompt_tokens", None)
            output_tokens = getattr(response.usage, "completion_tokens", None)

        try:
            cost = litellm.completion_cost(completion_response=response)
        except Exception:
            pass

        rule_votes = None
        if decision.scores:
            rule_votes = {k.value: v for k, v in decision.scores.items()}

        record = DecisionRecord(
            timestamp=datetime.now(timezone.utc),
            prompt_hash=prompt_hash,
            category=decision.category.value,
            confidence=decision.confidence,
            feature_type=decision.feature_type.value,
            selected_model=decision.model.name,
            used_model=used_model.name,
            response_time_ms=response_time_ms,
            input_tokens=input_tokens,
            output_tokens=output_tokens,
            cost=cost,
            fallback_triggered=decision.model.name != used_model.name,
            escalated=decision.escalated,
            rule_votes=rule_votes,
            embedding=embedding_bytes,
        )
        await repository.save(record)
    except Exception as e:
        logger.warning("Failed to log decision: %s", e)


async def handle_chat_completion(
    body: dict,
    engine: RoutingEngine,
    authorization: str | None = None,
    adapter: ClientAdapter | None = None,
    pipeline: EnrichmentPipeline | None = None,
    repository: DecisionRepository | None = None,
    embedding_service=None,
    scheduler=None,
) -> Response:
    stream = body.get("stream", False)
    request_api_key = _extract_bearer_token(authorization)

    active_adapter = adapter or _DEFAULT_ADAPTER
    normalized = active_adapter.normalize(
        messages=body.get("messages", []),
        max_tokens=body.get("max_tokens"),
        temperature=body.get("temperature"),
    )

    embedding = None
    embedding_bytes = None
    last_user_text = extract_last_user_text(normalized.messages)
    prompt_hash = _hash_prompt(last_user_text)

    if embedding_service is not None and last_user_text.strip():
        try:
            embedding = embedding_service.embed(last_user_text)
            embedding_bytes = embedding.tobytes()
        except Exception as e:
            logger.warning("Embedding failed: %s", e)

    decision = await engine.select_model(
        messages=normalized.messages,
        max_tokens=normalized.max_tokens,
        temperature=normalized.temperature,
        feature_type=normalized.feature_type,
        embedding=embedding,
    )

    if pipeline is not None:
        enrichment_ctx = EnrichmentContext(
            messages=list(body.get("messages", [])),
            category=decision.category,
            confidence=decision.confidence,
            feature_type=decision.feature_type,
        )
        enrichment_ctx = pipeline.run(enrichment_ctx)
        body = {**body, "messages": enrichment_ctx.messages}

    body = {**body, "messages": sanitize_messages(body.get("messages", []))}

    start_time = time.perf_counter()
    response, used_model = await _call_with_fallback(
        engine, decision.model, body, stream, request_api_key
    )
    response_time_ms = int((time.perf_counter() - start_time) * 1000)
    logger.info("Routed to %s", used_model.name)

    if repository is not None:
        asyncio.create_task(
            _log_decision(
                repository,
                decision,
                used_model,
                response_time_ms,
                response,
                prompt_hash,
                embedding_bytes,
            )
        )

    if scheduler is not None:
        asyncio.create_task(scheduler.on_new_decision())

    if stream:
        return StreamingResponse(
            stream_completion(response),
            media_type="text/event-stream",
        )
    return JSONResponse(content=response.model_dump())


async def handle_text_completion(
    body: dict, engine: RoutingEngine, authorization: str | None = None
) -> Response:
    stream = body.get("stream", False)
    request_api_key = _extract_bearer_token(authorization)

    decision = await engine.select_model(
        messages=body.get("messages", []),
        max_tokens=body.get("max_tokens"),
        temperature=body.get("temperature"),
    )

    response, used_model = await _call_with_fallback(
        engine, decision.model, body, stream, request_api_key
    )
    logger.info("Routed text completion to %s", used_model.name)

    if stream:
        return StreamingResponse(
            stream_completion(response),
            media_type="text/event-stream",
        )
    return JSONResponse(content=response.model_dump())


async def handle_anthropic_messages(
    body: dict,
    engine: RoutingEngine,
    request: Request,
    adapter: ClientAdapter | None = None,
    pipeline: EnrichmentPipeline | None = None,
    repository: DecisionRepository | None = None,
    embedding_service=None,
    scheduler=None,
) -> Response:
    stream = body.get("stream", False)
    request_api_key = extract_anthropic_api_key(request)
    request_model = body.get("model")
    openai_body = anthropic_to_openai(body)

    active_adapter = adapter or _DEFAULT_ADAPTER
    normalized = active_adapter.normalize(
        messages=openai_body.get("messages", []),
        max_tokens=openai_body.get("max_tokens"),
        temperature=openai_body.get("temperature"),
    )

    embedding = None
    embedding_bytes = None
    last_user_text = extract_last_user_text(normalized.messages)
    prompt_hash = _hash_prompt(last_user_text)

    if embedding_service is not None and last_user_text.strip():
        try:
            embedding = embedding_service.embed(last_user_text)
            embedding_bytes = embedding.tobytes()
        except Exception as e:
            logger.warning("Embedding failed: %s", e)

    decision = await engine.select_model(
        messages=normalized.messages,
        max_tokens=normalized.max_tokens,
        temperature=normalized.temperature,
        feature_type=normalized.feature_type,
        embedding=embedding,
    )

    if pipeline is not None:
        enrichment_ctx = EnrichmentContext(
            messages=list(openai_body.get("messages", [])),
            category=decision.category,
            confidence=decision.confidence,
            feature_type=decision.feature_type,
        )
        enrichment_ctx = pipeline.run(enrichment_ctx)
        openai_body = {**openai_body, "messages": enrichment_ctx.messages}

    start_time = time.perf_counter()
    response, used_model = await _call_with_fallback(
        engine, decision.model, openai_body, stream, request_api_key
    )
    response_time_ms = int((time.perf_counter() - start_time) * 1000)
    logger.info("Routed to %s (anthropic)", used_model.name)

    if repository is not None:
        asyncio.create_task(
            _log_decision(
                repository,
                decision,
                used_model,
                response_time_ms,
                response,
                prompt_hash,
                embedding_bytes,
            )
        )

    if scheduler is not None:
        asyncio.create_task(scheduler.on_new_decision())

    if stream:
        return StreamingResponse(
            stream_anthropic_response(response, used_model.name, request_model),
            media_type="text/event-stream",
        )

    anthropic_response = openai_response_to_anthropic(response, used_model.name, request_model)
    return JSONResponse(content=anthropic_response)


async def handle_passthrough(request: Request, api_base: str | None) -> Response:
    if not api_base:
        return JSONResponse(
            status_code=501,
            content={
                "error": {
                    "message": (
                        "Passthrough not available:" " no api_base configured for primary model"
                    ),
                    "type": "proxy_error",
                    "code": 501,
                }
            },
        )

    path = request.url.path
    async with httpx.AsyncClient(base_url=api_base) as client:
        upstream = await client.request(
            method=request.method,
            url=path,
            headers={
                k: v
                for k, v in request.headers.items()
                if k.lower() not in ("host", "content-length")
            },
            content=await request.body(),
            params=dict(request.query_params),
        )
    return Response(
        content=upstream.content,
        status_code=upstream.status_code,
        headers=dict(upstream.headers),
    )

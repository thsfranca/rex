from __future__ import annotations

import logging

import litellm
from fastapi import Request, Response
from fastapi.responses import JSONResponse, StreamingResponse

from app.config import ModelConfig
from app.proxy.streaming import stream_completion
from app.router.engine import RoutingEngine

logger = logging.getLogger(__name__)

LITELLM_PASSTHROUGH_PARAMS = [
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
]


def _build_litellm_params(body: dict, model_config: ModelConfig) -> dict:
    params = {"model": model_config.name}
    if model_config.api_key:
        params["api_key"] = model_config.api_key
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
):
    models_to_try = [primary] + engine.fallback_order(primary)
    last_error = None

    for model in models_to_try:
        try:
            params = _build_litellm_params(body, model)
            if stream:
                params["stream"] = True
            response = await litellm.acompletion(**params)
            return response, model
        except Exception as e:
            logger.warning("Model %s failed: %s", model.name, e)
            last_error = e

    raise last_error


async def handle_chat_completion(body: dict, engine: RoutingEngine) -> Response:
    stream = body.get("stream", False)

    selected = engine.select_model(
        messages=body.get("messages", []),
        max_tokens=body.get("max_tokens"),
        temperature=body.get("temperature"),
    )

    response, used_model = await _call_with_fallback(engine, selected, body, stream)
    logger.info("Routed to %s", used_model.name)

    if stream:
        return StreamingResponse(
            stream_completion(response),
            media_type="text/event-stream",
        )
    return JSONResponse(content=response.model_dump())


async def handle_text_completion(body: dict, engine: RoutingEngine) -> Response:
    stream = body.get("stream", False)

    completion_model_name = engine._routing_config.completion_model
    selected = engine._registry.get_by_name(completion_model_name)
    if selected is None:
        raise ValueError(f"Completion model '{completion_model_name}' not found in registry")

    response, used_model = await _call_with_fallback(engine, selected, body, stream)
    logger.info("Routed text completion to %s", used_model.name)

    if stream:
        return StreamingResponse(
            stream_completion(response),
            media_type="text/event-stream",
        )
    return JSONResponse(content=response.model_dump())


async def handle_passthrough(request: Request, api_base: str | None) -> Response:
    if not api_base:
        return JSONResponse(
            status_code=501,
            content={
                "error": {
                    "message": (
                        "Passthrough not available:"
                        " no api_base configured for default model"
                    ),
                    "type": "proxy_error",
                    "code": 501,
                }
            },
        )

    import httpx

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

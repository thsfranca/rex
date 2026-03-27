from __future__ import annotations

import logging
from contextlib import asynccontextmanager

from fastapi import FastAPI, Request
from fastapi.responses import JSONResponse

from app.adapters.registry import AdapterRegistry
from app.config import Settings, load_config
from app.discovery.registry_builder import build_registry
from app.enrichment.pipeline import EnrichmentPipeline, build_pipeline
from app.proxy.handler import (
    handle_chat_completion,
    handle_passthrough,
    handle_text_completion,
)
from app.router.engine import RoutingEngine

logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(name)s: %(message)s")
logger = logging.getLogger(__name__)

_engine: RoutingEngine | None = None
_settings: Settings | None = None
_adapter_registry: AdapterRegistry | None = None
_pipeline: EnrichmentPipeline | None = None


@asynccontextmanager
async def lifespan(app: FastAPI):
    global _engine, _settings, _adapter_registry, _pipeline
    config = load_config("config.yaml")
    registry, _settings = await build_registry(config)
    primary_model = _settings.routing.primary_model
    _engine = RoutingEngine(registry, primary_model)
    _adapter_registry = AdapterRegistry()
    _pipeline = build_pipeline(_settings)
    logger.info(
        "Rex started with %d models, primary: %s",
        len(registry.get_all()),
        _engine.primary.name,
    )
    yield


app = FastAPI(title="Rex", lifespan=lifespan)


def _get_engine() -> RoutingEngine:
    if _engine is None:
        raise RuntimeError("RoutingEngine not initialized")
    return _engine


def _get_adapter_registry() -> AdapterRegistry:
    if _adapter_registry is None:
        raise RuntimeError("AdapterRegistry not initialized")
    return _adapter_registry


def _get_settings() -> Settings:
    if _settings is None:
        raise RuntimeError("Settings not initialized")
    return _settings


def _get_pipeline() -> EnrichmentPipeline:
    if _pipeline is None:
        raise RuntimeError("EnrichmentPipeline not initialized")
    return _pipeline


def _error_response(status_code: int, message: str, error_type: str) -> JSONResponse:
    return JSONResponse(
        status_code=status_code,
        content={"error": {"message": message, "type": error_type, "code": status_code}},
    )


@app.post("/v1/chat/completions")
async def chat_completions(request: Request):
    body = await request.json()
    authorization = request.headers.get("authorization")
    user_agent = request.headers.get("user-agent")
    adapter = _get_adapter_registry().get_adapter(user_agent)
    try:
        return await handle_chat_completion(
            body, _get_engine(), authorization, adapter, _get_pipeline()
        )
    except Exception as e:
        logger.exception("Chat completion failed")
        return _error_response(502, f"All model backends failed. Last error: {e}", "proxy_error")


@app.post("/v1/completions")
async def text_completions(request: Request):
    body = await request.json()
    authorization = request.headers.get("authorization")
    try:
        return await handle_text_completion(body, _get_engine(), authorization)
    except Exception as e:
        logger.exception("Text completion failed")
        return _error_response(502, f"All model backends failed. Last error: {e}", "proxy_error")


@app.get("/v1/models")
async def list_models():
    engine = _get_engine()
    models = engine._registry.get_all()
    return {
        "object": "list",
        "data": [
            {
                "id": m.name,
                "object": "model",
                "created": 0,
                "owned_by": "rex",
            }
            for m in models
        ],
    }


@app.get("/health")
async def health():
    return {"status": "ok"}


@app.api_route("/{path:path}", methods=["GET", "POST", "PUT", "DELETE", "PATCH"])
async def passthrough(request: Request, path: str):
    engine = _get_engine()
    api_base = engine.primary.api_base
    return await handle_passthrough(request, api_base)

from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock, patch

import pytest
from fastapi.responses import JSONResponse, StreamingResponse

from app.config import ModelConfig, RoutingConfig
from app.proxy.handler import handle_chat_completion, handle_text_completion
from app.router.engine import RoutingEngine
from app.router.registry import ModelRegistry


def _make_model(**overrides) -> ModelConfig:
    defaults = {
        "name": "test/model",
        "provider": "test",
        "context_window": 4096,
        "cost_per_1k_input": 0.001,
        "cost_per_1k_output": 0.002,
        "strengths": ["general"],
        "max_latency_ms": 500,
        "is_local": False,
    }
    defaults.update(overrides)
    return ModelConfig(**defaults)


def _make_engine(
    models: list[ModelConfig],
    completion_model: str,
    default_model: str,
) -> RoutingEngine:
    registry = ModelRegistry(models)
    routing = RoutingConfig(completion_model=completion_model, default_model=default_model)
    return RoutingEngine(registry, routing)


class FakeResponse:
    def model_dump(self) -> dict:
        return {"id": "resp-1", "choices": [{"message": {"content": "hello"}}]}


class TestHandleChatCompletion:
    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_non_streaming_returns_json(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(return_value=FakeResponse())
        model = _make_model(name="test/model")
        engine = _make_engine([model], "test/model", "test/model")

        body = {"messages": [{"role": "user", "content": "hello"}]}
        response = await handle_chat_completion(body, engine)

        assert isinstance(response, JSONResponse)
        mock_litellm.acompletion.assert_awaited_once()

    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_streaming_returns_sse(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(return_value=MagicMock())
        model = _make_model(name="test/model")
        engine = _make_engine([model], "test/model", "test/model")

        body = {"messages": [{"role": "user", "content": "hello"}], "stream": True}
        response = await handle_chat_completion(body, engine)

        assert isinstance(response, StreamingResponse)
        assert response.media_type == "text/event-stream"

    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_passes_litellm_params(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(return_value=FakeResponse())
        model = _make_model(name="test/model", api_key="sk-test", api_base="http://local:8080")
        engine = _make_engine([model], "test/model", "test/model")

        body = {
            "messages": [{"role": "user", "content": "hello"}],
            "temperature": 0.5,
            "max_tokens": 100,
        }
        await handle_chat_completion(body, engine)

        call_kwargs = mock_litellm.acompletion.call_args.kwargs
        assert call_kwargs["model"] == "test/model"
        assert call_kwargs["api_key"] == "sk-test"
        assert call_kwargs["api_base"] == "http://local:8080"
        assert call_kwargs["temperature"] == 0.5
        assert call_kwargs["max_tokens"] == 100

    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_fallback_on_primary_failure(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(
            side_effect=[Exception("primary failed"), FakeResponse()]
        )
        primary = _make_model(name="primary/model")
        fallback = _make_model(name="fallback/model")
        engine = _make_engine([primary, fallback], "primary/model", "fallback/model")

        body = {"messages": [{"role": "user", "content": "hello"}]}
        response = await handle_chat_completion(body, engine)

        assert isinstance(response, JSONResponse)
        assert mock_litellm.acompletion.await_count == 2

    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_all_models_fail_raises(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(side_effect=Exception("all failed"))
        model = _make_model(name="only/model")
        engine = _make_engine([model], "only/model", "only/model")

        body = {"messages": [{"role": "user", "content": "hello"}]}
        with pytest.raises(Exception, match="all failed"):
            await handle_chat_completion(body, engine)


class TestHandleTextCompletion:
    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_routes_to_completion_model(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(return_value=FakeResponse())
        fast = _make_model(name="fast/model")
        strong = _make_model(name="strong/model")
        engine = _make_engine([fast, strong], "fast/model", "strong/model")

        body = {"messages": [{"role": "user", "content": "complete this"}]}
        await handle_text_completion(body, engine)

        call_kwargs = mock_litellm.acompletion.call_args.kwargs
        assert call_kwargs["model"] == "fast/model"

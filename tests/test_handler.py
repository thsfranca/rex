from __future__ import annotations

import asyncio
from unittest.mock import AsyncMock, MagicMock, patch

import pytest
from fastapi.responses import JSONResponse, StreamingResponse

from app.config import ModelConfig
from app.proxy.handler import (
    _extract_bearer_token,
    _hash_prompt,
    handle_chat_completion,
    handle_text_completion,
)
from app.router.engine import RoutingEngine
from app.router.registry import ModelRegistry


def _make_model(**overrides) -> ModelConfig:
    defaults = {"name": "test/model"}
    defaults.update(overrides)
    return ModelConfig(**defaults)


def _make_engine(
    models: list[ModelConfig],
    primary_model: str | None = None,
) -> RoutingEngine:
    registry = ModelRegistry(models)
    return RoutingEngine(registry, primary_model)


class FakeResponse:
    def model_dump(self) -> dict:
        return {"id": "resp-1", "choices": [{"message": {"content": "hello"}}]}


class TestHandleChatCompletion:
    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_non_streaming_returns_json(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(return_value=FakeResponse())
        model = _make_model(name="test/model")
        engine = _make_engine([model])

        body = {"messages": [{"role": "user", "content": "hello"}]}
        response = await handle_chat_completion(body, engine)

        assert isinstance(response, JSONResponse)
        mock_litellm.acompletion.assert_awaited_once()

    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_streaming_returns_sse(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(return_value=MagicMock())
        model = _make_model(name="test/model")
        engine = _make_engine([model])

        body = {"messages": [{"role": "user", "content": "hello"}], "stream": True}
        response = await handle_chat_completion(body, engine)

        assert isinstance(response, StreamingResponse)
        assert response.media_type == "text/event-stream"

    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_passes_litellm_params(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(return_value=FakeResponse())
        model = _make_model(name="test/model", api_key="sk-test", api_base="http://local:8080")
        engine = _make_engine([model])

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
        primary = _make_model(name="primary/model", cost_per_1k_input=0.001)
        fallback = _make_model(name="fallback/model", cost_per_1k_input=0.01)
        engine = _make_engine([primary, fallback])

        body = {"messages": [{"role": "user", "content": "hello"}]}
        response = await handle_chat_completion(body, engine)

        assert isinstance(response, JSONResponse)
        assert mock_litellm.acompletion.await_count == 2

    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_all_models_fail_raises(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(side_effect=Exception("all failed"))
        model = _make_model(name="only/model")
        engine = _make_engine([model])

        body = {"messages": [{"role": "user", "content": "hello"}]}
        with pytest.raises(Exception, match="all failed"):
            await handle_chat_completion(body, engine)

    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_uses_request_api_key_when_config_has_none(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(return_value=FakeResponse())
        model = _make_model(name="test/model", api_key=None)
        engine = _make_engine([model])

        body = {"messages": [{"role": "user", "content": "hello"}]}
        await handle_chat_completion(body, engine, authorization="Bearer sk-from-client")

        call_kwargs = mock_litellm.acompletion.call_args.kwargs
        assert call_kwargs["api_key"] == "sk-from-client"

    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_config_api_key_overrides_request_key(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(return_value=FakeResponse())
        model = _make_model(name="test/model", api_key="sk-from-config")
        engine = _make_engine([model])

        body = {"messages": [{"role": "user", "content": "hello"}]}
        await handle_chat_completion(body, engine, authorization="Bearer sk-from-client")

        call_kwargs = mock_litellm.acompletion.call_args.kwargs
        assert call_kwargs["api_key"] == "sk-from-config"

    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_no_api_key_anywhere(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(return_value=FakeResponse())
        model = _make_model(name="test/model", api_key=None)
        engine = _make_engine([model])

        body = {"messages": [{"role": "user", "content": "hello"}]}
        await handle_chat_completion(body, engine)

        call_kwargs = mock_litellm.acompletion.call_args.kwargs
        assert "api_key" not in call_kwargs


class TestExtractBearerToken:
    def test_extracts_bearer_token(self):
        assert _extract_bearer_token("Bearer sk-abc123") == "sk-abc123"

    def test_returns_none_for_no_header(self):
        assert _extract_bearer_token(None) is None

    def test_returns_none_for_non_bearer(self):
        assert _extract_bearer_token("Basic dXNlcjpwYXNz") is None


class TestDecisionLogging:
    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_logs_decision_when_repository_provided(self, mock_litellm):
        fake_response = FakeResponse()
        fake_response.usage = MagicMock(prompt_tokens=10, completion_tokens=20)
        mock_litellm.acompletion = AsyncMock(return_value=fake_response)
        mock_litellm.completion_cost.return_value = 0.001

        model = _make_model(name="test/model")
        engine = _make_engine([model])

        repository = MagicMock()
        repository.save = AsyncMock()

        body = {"messages": [{"role": "user", "content": "hello"}]}
        await handle_chat_completion(body, engine, repository=repository)
        await asyncio.sleep(0.05)

        repository.save.assert_called_once()
        record = repository.save.call_args[0][0]
        assert record.selected_model == "test/model"
        assert record.used_model == "test/model"
        assert record.category == "general"
        assert record.prompt_hash == _hash_prompt("hello")
        assert record.input_tokens == 10
        assert record.output_tokens == 20

    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_no_logging_when_no_repository(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(return_value=FakeResponse())
        model = _make_model(name="test/model")
        engine = _make_engine([model])

        body = {"messages": [{"role": "user", "content": "hello"}]}
        response = await handle_chat_completion(body, engine)
        assert isinstance(response, JSONResponse)

    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_logging_failure_does_not_break_response(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(return_value=FakeResponse())
        model = _make_model(name="test/model")
        engine = _make_engine([model])

        repository = MagicMock()
        repository.save = AsyncMock(side_effect=Exception("db error"))

        body = {"messages": [{"role": "user", "content": "hello"}]}
        response = await handle_chat_completion(body, engine, repository=repository)
        await asyncio.sleep(0.05)

        assert isinstance(response, JSONResponse)

    def test_hash_prompt_deterministic(self):
        assert _hash_prompt("test") == _hash_prompt("test")
        assert _hash_prompt("test") != _hash_prompt("other")


class TestHandleTextCompletion:
    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_routes_to_primary_model(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(return_value=FakeResponse())
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001)
        expensive = _make_model(name="expensive/model", cost_per_1k_input=0.03)
        engine = _make_engine([cheap, expensive])

        body = {"messages": [{"role": "user", "content": "complete this"}]}
        await handle_text_completion(body, engine)

        call_kwargs = mock_litellm.acompletion.call_args.kwargs
        assert call_kwargs["model"] == "cheap/model"

    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_uses_request_api_key(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(return_value=FakeResponse())
        model = _make_model(name="test/model", api_key=None)
        engine = _make_engine([model])

        body = {"messages": [{"role": "user", "content": "complete this"}]}
        await handle_text_completion(body, engine, authorization="Bearer sk-req")

        call_kwargs = mock_litellm.acompletion.call_args.kwargs
        assert call_kwargs["api_key"] == "sk-req"

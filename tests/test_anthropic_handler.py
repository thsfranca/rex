from __future__ import annotations

import asyncio
from unittest.mock import AsyncMock, MagicMock, patch

import pytest
from fastapi.responses import JSONResponse

from app.config import Model
from app.proxy.handler import handle_anthropic_messages
from app.router.engine import RoutingEngine
from app.router.registry import ModelRegistry


def _make_model(**overrides) -> Model:
    defaults = {"name": "test/model"}
    defaults.update(overrides)
    return Model(**defaults)


def _make_engine(models: list[Model], primary_model: str | None = None) -> RoutingEngine:
    registry = ModelRegistry(models)
    return RoutingEngine(registry, primary_model)


def _make_request(api_key: str | None = None, user_agent: str | None = None) -> MagicMock:
    request = MagicMock()
    headers = {}
    if api_key:
        headers["x-api-key"] = api_key
    if user_agent:
        headers["user-agent"] = user_agent
    request.headers = headers
    return request


class FakeResponse:
    def __init__(self, content="Hello!", finish_reason="stop"):
        choice = MagicMock()
        choice.message.content = content
        choice.finish_reason = finish_reason
        self.choices = [choice]
        self.usage = MagicMock(prompt_tokens=10, completion_tokens=5)

    def model_dump(self) -> dict:
        return {"id": "resp-1", "choices": [{"message": {"content": "Hello!"}}]}


class TestHandleAnthropicMessages:
    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_returns_anthropic_format(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(return_value=FakeResponse())
        model = _make_model(name="test/model")
        engine = _make_engine([model])
        request = _make_request()

        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}],
        }
        response = await handle_anthropic_messages(body, engine, request)

        assert isinstance(response, JSONResponse)
        import json

        data = json.loads(response.body.decode())
        assert data["type"] == "message"
        assert data["role"] == "assistant"
        assert data["stop_reason"] == "end_turn"
        assert data["content"][0]["type"] == "text"

    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_echoes_request_model_in_response(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(return_value=FakeResponse())
        model = _make_model(name="ollama/llama3")
        engine = _make_engine([model])
        request = _make_request()

        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}],
        }
        response = await handle_anthropic_messages(body, engine, request)

        content = response.body.decode()
        assert '"claude-3-sonnet"' in content

    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_translates_system_prompt(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(return_value=FakeResponse())
        model = _make_model(name="test/model")
        engine = _make_engine([model])
        request = _make_request()

        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "system": "You are a coding assistant.",
            "messages": [{"role": "user", "content": "Hello"}],
        }
        await handle_anthropic_messages(body, engine, request)

        call_kwargs = mock_litellm.acompletion.call_args.kwargs
        messages = call_kwargs["messages"]
        assert messages[0]["role"] == "system"
        assert messages[0]["content"] == "You are a coding assistant."
        assert messages[1]["role"] == "user"

    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_passes_api_key_from_x_api_key_header(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(return_value=FakeResponse())
        model = _make_model(name="test/model", api_key=None)
        engine = _make_engine([model])
        request = _make_request(api_key="sk-ant-test-key")

        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}],
        }
        await handle_anthropic_messages(body, engine, request)

        call_kwargs = mock_litellm.acompletion.call_args.kwargs
        assert call_kwargs["api_key"] == "sk-ant-test-key"

    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_fallback_on_failure(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(
            side_effect=[Exception("primary failed"), FakeResponse()]
        )
        primary = _make_model(name="primary/model", cost_per_1k_input=0.001)
        fallback = _make_model(name="fallback/model", cost_per_1k_input=0.01)
        engine = _make_engine([primary, fallback])
        request = _make_request()

        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}],
        }
        response = await handle_anthropic_messages(body, engine, request)

        assert isinstance(response, JSONResponse)
        assert mock_litellm.acompletion.await_count == 2

    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_all_models_fail_raises(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(side_effect=Exception("all failed"))
        model = _make_model(name="only/model")
        engine = _make_engine([model])
        request = _make_request()

        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}],
        }
        with pytest.raises(Exception, match="all failed"):
            await handle_anthropic_messages(body, engine, request)

    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_logs_decision(self, mock_litellm):
        fake_response = FakeResponse()
        mock_litellm.acompletion = AsyncMock(return_value=fake_response)
        mock_litellm.completion_cost.return_value = 0.001

        model = _make_model(name="test/model")
        engine = _make_engine([model])
        request = _make_request()

        repository = MagicMock()
        repository.save = AsyncMock()

        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}],
        }
        await handle_anthropic_messages(body, engine, request, repository=repository)
        await asyncio.sleep(0.05)

        repository.save.assert_called_once()
        record = repository.save.call_args[0][0]
        assert record.used_model == "test/model"

    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_routes_based_on_task_not_requested_model(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(return_value=FakeResponse())
        model = _make_model(name="ollama/qwen2.5-coder:7b")
        engine = _make_engine([model])
        request = _make_request()

        body = {
            "model": "claude-opus-4-20250514",
            "max_tokens": 4096,
            "messages": [{"role": "user", "content": "Explain what a for loop is"}],
        }
        await handle_anthropic_messages(body, engine, request)

        call_kwargs = mock_litellm.acompletion.call_args.kwargs
        assert call_kwargs["model"] == "ollama/qwen2.5-coder:7b"

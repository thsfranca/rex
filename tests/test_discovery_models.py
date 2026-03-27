from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock, patch

import httpx
import pytest

from app.discovery.models import list_models_for_provider
from app.discovery.providers import DetectedProvider


def _mock_httpx_client(response):
    mock_client = MagicMock()
    mock_client.get = AsyncMock(return_value=response)
    mock_client.__aenter__ = AsyncMock(return_value=mock_client)
    mock_client.__aexit__ = AsyncMock(return_value=False)
    return mock_client


def _mock_response(status_code, json_data):
    resp = MagicMock()
    resp.status_code = status_code
    resp.json.return_value = json_data
    return resp


class TestListOllamaModels:
    @pytest.mark.asyncio
    async def test_lists_ollama_models(self):
        response = _mock_response(
            200,
            {
                "models": [
                    {"name": "llama3:latest"},
                    {"name": "codellama:7b"},
                ]
            },
        )

        with patch("app.discovery.models.httpx.AsyncClient") as mock_cls:
            mock_cls.return_value = _mock_httpx_client(response)
            provider = DetectedProvider(
                prefix="ollama", api_base="http://localhost:11434", is_local=True
            )
            models = await list_models_for_provider(provider)

        assert models == ["ollama/llama3:latest", "ollama/codellama:7b"]

    @pytest.mark.asyncio
    async def test_returns_empty_on_failure(self):
        response = _mock_response(500, {})

        with patch("app.discovery.models.httpx.AsyncClient") as mock_cls:
            mock_cls.return_value = _mock_httpx_client(response)
            provider = DetectedProvider(
                prefix="ollama", api_base="http://localhost:11434", is_local=True
            )
            models = await list_models_for_provider(provider)

        assert models == []

    @pytest.mark.asyncio
    async def test_returns_empty_on_connection_error(self):
        mock_client = MagicMock()
        mock_client.get = AsyncMock(side_effect=httpx.ConnectError("refused"))
        mock_client.__aenter__ = AsyncMock(return_value=mock_client)
        mock_client.__aexit__ = AsyncMock(return_value=False)

        with patch("app.discovery.models.httpx.AsyncClient") as mock_cls:
            mock_cls.return_value = mock_client
            provider = DetectedProvider(
                prefix="ollama", api_base="http://localhost:11434", is_local=True
            )
            models = await list_models_for_provider(provider)

        assert models == []


class TestListOpenAICompatible:
    @pytest.mark.asyncio
    async def test_lists_openai_models(self):
        response = _mock_response(200, {"data": [{"id": "gpt-4o"}, {"id": "gpt-4o-mini"}]})

        with patch("app.discovery.models.httpx.AsyncClient") as mock_cls:
            mock_cls.return_value = _mock_httpx_client(response)
            provider = DetectedProvider(prefix="openai", api_key="sk-test")
            models = await list_models_for_provider(provider)

        assert models == ["openai/gpt-4o", "openai/gpt-4o-mini"]

    @pytest.mark.asyncio
    async def test_returns_empty_on_auth_failure(self):
        response = _mock_response(401, {})

        with patch("app.discovery.models.httpx.AsyncClient") as mock_cls:
            mock_cls.return_value = _mock_httpx_client(response)
            provider = DetectedProvider(prefix="openai", api_key="bad-key")
            models = await list_models_for_provider(provider)

        assert models == []

    @pytest.mark.asyncio
    async def test_lists_groq_models(self):
        response = _mock_response(200, {"data": [{"id": "llama-3.3-70b-versatile"}]})

        with patch("app.discovery.models.httpx.AsyncClient") as mock_cls:
            mock_cls.return_value = _mock_httpx_client(response)
            provider = DetectedProvider(prefix="groq", api_key="gsk-test")
            models = await list_models_for_provider(provider)

        assert models == ["groq/llama-3.3-70b-versatile"]


class TestListAnthropicModels:
    @pytest.mark.asyncio
    async def test_lists_anthropic_models(self):
        response = _mock_response(
            200,
            {
                "data": [
                    {"id": "claude-sonnet-4-20250514"},
                    {"id": "claude-3-5-haiku-20241022"},
                ]
            },
        )

        with patch("app.discovery.models.httpx.AsyncClient") as mock_cls:
            mock_cls.return_value = _mock_httpx_client(response)
            provider = DetectedProvider(prefix="anthropic", api_key="sk-ant-test")
            models = await list_models_for_provider(provider)

        assert models == [
            "anthropic/claude-sonnet-4-20250514",
            "anthropic/claude-3-5-haiku-20241022",
        ]


class TestListGeminiModels:
    @pytest.mark.asyncio
    async def test_lists_gemini_models(self):
        response = _mock_response(
            200,
            {
                "models": [
                    {"name": "models/gemini-2.0-flash"},
                    {"name": "models/gemini-1.5-pro"},
                ]
            },
        )

        with patch("app.discovery.models.httpx.AsyncClient") as mock_cls:
            mock_cls.return_value = _mock_httpx_client(response)
            provider = DetectedProvider(prefix="gemini", api_key="key-gemini")
            models = await list_models_for_provider(provider)

        assert models == ["gemini/gemini-2.0-flash", "gemini/gemini-1.5-pro"]


class TestUnknownProvider:
    @pytest.mark.asyncio
    async def test_returns_empty_for_unknown(self):
        provider = DetectedProvider(prefix="unknown_provider", api_key="key")
        models = await list_models_for_provider(provider)
        assert models == []

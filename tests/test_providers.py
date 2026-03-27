from __future__ import annotations

from unittest.mock import AsyncMock, patch

import httpx
import pytest

from app.discovery.providers import (
    KNOWN_PROVIDERS,
    detect_cloud_providers,
    detect_providers,
    probe_ollama,
)


class TestDetectCloudProviders:
    def test_detects_openai_key(self, monkeypatch):
        monkeypatch.setenv("OPENAI_API_KEY", "sk-test-123")
        providers = detect_cloud_providers()
        assert len(providers) == 1
        assert providers[0].prefix == "openai"
        assert providers[0].api_key == "sk-test-123"
        assert providers[0].is_local is False

    def test_detects_multiple_providers(self, monkeypatch):
        monkeypatch.setenv("OPENAI_API_KEY", "sk-openai")
        monkeypatch.setenv("ANTHROPIC_API_KEY", "sk-anthropic")
        providers = detect_cloud_providers()
        prefixes = {p.prefix for p in providers}
        assert "openai" in prefixes
        assert "anthropic" in prefixes

    def test_ignores_missing_env_vars(self, monkeypatch):
        for env_var in KNOWN_PROVIDERS:
            monkeypatch.delenv(env_var, raising=False)
        providers = detect_cloud_providers()
        assert providers == []

    def test_detects_all_known_providers(self, monkeypatch):
        for env_var in KNOWN_PROVIDERS:
            monkeypatch.setenv(env_var, f"key-for-{env_var}")
        providers = detect_cloud_providers()
        assert len(providers) == len(KNOWN_PROVIDERS)


class TestProbeOllama:
    @pytest.mark.asyncio
    async def test_detects_running_ollama(self):
        mock_response = AsyncMock()
        mock_response.status_code = 200

        with patch("app.discovery.providers.httpx.AsyncClient") as mock_client_cls:
            mock_client = AsyncMock()
            mock_client.get = AsyncMock(return_value=mock_response)
            mock_client.__aenter__ = AsyncMock(return_value=mock_client)
            mock_client.__aexit__ = AsyncMock(return_value=False)
            mock_client_cls.return_value = mock_client

            result = await probe_ollama()

        assert result is not None
        assert result.prefix == "ollama"
        assert result.is_local is True
        assert result.api_base == "http://localhost:11434"

    @pytest.mark.asyncio
    async def test_returns_none_when_ollama_not_running(self):
        with patch("app.discovery.providers.httpx.AsyncClient") as mock_client_cls:
            mock_client = AsyncMock()
            mock_client.get = AsyncMock(side_effect=httpx.ConnectError("refused"))
            mock_client.__aenter__ = AsyncMock(return_value=mock_client)
            mock_client.__aexit__ = AsyncMock(return_value=False)
            mock_client_cls.return_value = mock_client

            result = await probe_ollama()

        assert result is None

    @pytest.mark.asyncio
    async def test_returns_none_on_non_200(self):
        mock_response = AsyncMock()
        mock_response.status_code = 500

        with patch("app.discovery.providers.httpx.AsyncClient") as mock_client_cls:
            mock_client = AsyncMock()
            mock_client.get = AsyncMock(return_value=mock_response)
            mock_client.__aenter__ = AsyncMock(return_value=mock_client)
            mock_client.__aexit__ = AsyncMock(return_value=False)
            mock_client_cls.return_value = mock_client

            result = await probe_ollama()

        assert result is None

    @pytest.mark.asyncio
    async def test_custom_base_url(self):
        mock_response = AsyncMock()
        mock_response.status_code = 200

        with patch("app.discovery.providers.httpx.AsyncClient") as mock_client_cls:
            mock_client = AsyncMock()
            mock_client.get = AsyncMock(return_value=mock_response)
            mock_client.__aenter__ = AsyncMock(return_value=mock_client)
            mock_client.__aexit__ = AsyncMock(return_value=False)
            mock_client_cls.return_value = mock_client

            result = await probe_ollama("http://192.168.1.100:11434")

        assert result is not None
        assert result.api_base == "http://192.168.1.100:11434"


class TestDetectProviders:
    @pytest.mark.asyncio
    async def test_combines_cloud_and_local(self, monkeypatch):
        monkeypatch.setenv("OPENAI_API_KEY", "sk-test")
        for env_var in KNOWN_PROVIDERS:
            if env_var != "OPENAI_API_KEY":
                monkeypatch.delenv(env_var, raising=False)

        mock_response = AsyncMock()
        mock_response.status_code = 200

        with patch("app.discovery.providers.httpx.AsyncClient") as mock_client_cls:
            mock_client = AsyncMock()
            mock_client.get = AsyncMock(return_value=mock_response)
            mock_client.__aenter__ = AsyncMock(return_value=mock_client)
            mock_client.__aexit__ = AsyncMock(return_value=False)
            mock_client_cls.return_value = mock_client

            providers = await detect_providers()

        prefixes = {p.prefix for p in providers}
        assert "openai" in prefixes
        assert "ollama" in prefixes
        assert len(providers) == 2

    @pytest.mark.asyncio
    async def test_cloud_only_when_ollama_down(self, monkeypatch):
        monkeypatch.setenv("OPENAI_API_KEY", "sk-test")
        for env_var in KNOWN_PROVIDERS:
            if env_var != "OPENAI_API_KEY":
                monkeypatch.delenv(env_var, raising=False)

        with patch("app.discovery.providers.httpx.AsyncClient") as mock_client_cls:
            mock_client = AsyncMock()
            mock_client.get = AsyncMock(side_effect=httpx.ConnectError("refused"))
            mock_client.__aenter__ = AsyncMock(return_value=mock_client)
            mock_client.__aexit__ = AsyncMock(return_value=False)
            mock_client_cls.return_value = mock_client

            providers = await detect_providers()

        assert len(providers) == 1
        assert providers[0].prefix == "openai"

    @pytest.mark.asyncio
    async def test_returns_empty_when_nothing_found(self, monkeypatch):
        for env_var in KNOWN_PROVIDERS:
            monkeypatch.delenv(env_var, raising=False)

        with patch("app.discovery.providers.httpx.AsyncClient") as mock_client_cls:
            mock_client = AsyncMock()
            mock_client.get = AsyncMock(side_effect=httpx.ConnectError("refused"))
            mock_client.__aenter__ = AsyncMock(return_value=mock_client)
            mock_client.__aexit__ = AsyncMock(return_value=False)
            mock_client_cls.return_value = mock_client

            providers = await detect_providers()

        assert providers == []

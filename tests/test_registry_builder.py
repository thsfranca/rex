from __future__ import annotations

from unittest.mock import AsyncMock, patch

import pytest

from app.config import Model, ModelConfig, ProviderConfig, Settings
from app.discovery.providers import DetectedProvider
from app.discovery.registry_builder import _resolve_config_providers, build_registry


def _mock_detect_providers(providers):
    return AsyncMock(return_value=providers)


def _mock_list_models(model_map):
    async def _list(provider):
        return model_map.get(provider.prefix, [])

    return _list


def _mock_enrich(name, provider):
    return Model(
        name=name,
        api_key=provider.api_key,
        api_base=provider.api_base,
        cost_per_1k_input=0.001 if not provider.is_local else 0.0,
        is_local=provider.is_local,
    )


class TestBuildRegistry:
    @pytest.mark.asyncio
    @patch("app.discovery.registry_builder.detect_providers")
    @patch("app.discovery.registry_builder.list_models_for_provider")
    @patch("app.discovery.registry_builder.enrich_model")
    async def test_discovers_models_from_providers(self, mock_enrich, mock_list, mock_detect):
        mock_detect.return_value = [DetectedProvider(prefix="openai", api_key="sk-test")]
        mock_list.side_effect = _mock_list_models(
            {"openai": ["openai/gpt-4o", "openai/gpt-4o-mini"]}
        )
        mock_enrich.side_effect = _mock_enrich

        registry, settings = await build_registry(None)

        assert len(registry.get_all()) == 2
        assert registry.get_by_name("openai/gpt-4o") is not None
        assert registry.get_by_name("openai/gpt-4o-mini") is not None

    @pytest.mark.asyncio
    @patch("app.discovery.registry_builder.detect_providers")
    @patch("app.discovery.registry_builder.list_models_for_provider")
    @patch("app.discovery.registry_builder.enrich_model")
    async def test_config_takes_priority_over_discovered_model(
        self, mock_enrich, mock_list, mock_detect
    ):
        mock_detect.return_value = [DetectedProvider(prefix="openai", api_key="sk-env")]
        mock_list.side_effect = _mock_list_models({"openai": ["openai/gpt-4o"]})
        mock_enrich.side_effect = _mock_enrich

        override = ModelConfig(name="openai/gpt-4o", api_key="sk-override")
        config = Settings(models=[override])

        registry, _ = await build_registry(config)

        model = registry.get_by_name("openai/gpt-4o")
        assert model.api_key == "sk-override"

    @pytest.mark.asyncio
    @patch("app.discovery.registry_builder.detect_providers")
    @patch("app.discovery.registry_builder.list_models_for_provider")
    @patch("app.discovery.registry_builder.enrich_model")
    async def test_discovery_does_not_enrich_config_defined_models(
        self, mock_enrich, mock_list, mock_detect
    ):
        mock_detect.return_value = [DetectedProvider(prefix="openai", api_key="sk-env")]
        mock_list.side_effect = _mock_list_models({"openai": ["openai/gpt-4o"]})
        mock_enrich.side_effect = _mock_enrich

        config_model = ModelConfig(name="openai/gpt-4o", api_key="sk-config")
        config = Settings(models=[config_model])

        await build_registry(config)

        mock_enrich.assert_not_called()

    @pytest.mark.asyncio
    @patch("app.discovery.registry_builder.detect_providers")
    @patch("app.discovery.registry_builder.list_models_for_provider")
    @patch("app.discovery.registry_builder.enrich_model")
    async def test_config_adds_manual_models(self, mock_enrich, mock_list, mock_detect):
        mock_detect.return_value = [DetectedProvider(prefix="openai", api_key="sk-test")]
        mock_list.side_effect = _mock_list_models({"openai": ["openai/gpt-4o"]})
        mock_enrich.side_effect = _mock_enrich

        manual = ModelConfig(name="custom/model", api_base="http://internal:8080")
        config = Settings(models=[manual])

        registry, _ = await build_registry(config)

        assert registry.get_by_name("openai/gpt-4o") is not None
        assert registry.get_by_name("custom/model") is not None
        assert len(registry.get_all()) == 2

    @pytest.mark.asyncio
    @patch("app.discovery.registry_builder.detect_providers")
    @patch("app.discovery.registry_builder.list_models_for_provider")
    async def test_exits_when_no_models_found(self, mock_list, mock_detect):
        mock_detect.return_value = []
        mock_list.return_value = []

        with pytest.raises(SystemExit):
            await build_registry(None)

    @pytest.mark.asyncio
    @patch("app.discovery.registry_builder.detect_providers")
    @patch("app.discovery.registry_builder.list_models_for_provider")
    async def test_config_only_models_no_discovery(self, mock_list, mock_detect):
        mock_detect.return_value = []

        manual = ModelConfig(name="custom/model", api_key="sk-custom")
        config = Settings(models=[manual])

        registry, _ = await build_registry(config)

        assert len(registry.get_all()) == 1
        assert registry.get_by_name("custom/model") is not None

    @pytest.mark.asyncio
    @patch("app.discovery.registry_builder.detect_providers")
    @patch("app.discovery.registry_builder.list_models_for_provider")
    @patch("app.discovery.registry_builder.enrich_model")
    async def test_multiple_providers(self, mock_enrich, mock_list, mock_detect):
        mock_detect.return_value = [
            DetectedProvider(prefix="openai", api_key="sk-openai"),
            DetectedProvider(
                prefix="ollama",
                api_base="http://localhost:11434",
                is_local=True,
            ),
        ]
        mock_list.side_effect = _mock_list_models(
            {
                "openai": ["openai/gpt-4o"],
                "ollama": ["ollama/llama3"],
            }
        )
        mock_enrich.side_effect = _mock_enrich

        registry, _ = await build_registry(None)

        assert len(registry.get_all()) == 2
        local = registry.get_by_name("ollama/llama3")
        assert local.is_local is True
        cloud = registry.get_by_name("openai/gpt-4o")
        assert cloud.is_local is False

    @pytest.mark.asyncio
    @patch("app.discovery.registry_builder.detect_providers")
    @patch("app.discovery.registry_builder.list_models_for_provider")
    @patch("app.discovery.registry_builder.enrich_model")
    async def test_returns_settings(self, mock_enrich, mock_list, mock_detect):
        mock_detect.return_value = [DetectedProvider(prefix="openai", api_key="sk-test")]
        mock_list.side_effect = _mock_list_models({"openai": ["openai/gpt-4o"]})
        mock_enrich.side_effect = _mock_enrich

        _, settings = await build_registry(None)

        assert isinstance(settings, Settings)

    @pytest.mark.asyncio
    @patch("app.discovery.registry_builder.detect_providers")
    @patch("app.discovery.registry_builder.list_models_for_provider")
    @patch("app.discovery.registry_builder.enrich_model")
    async def test_config_provider_discovers_models(self, mock_enrich, mock_list, mock_detect):
        mock_detect.return_value = []
        mock_list.side_effect = _mock_list_models(
            {"anthropic": ["anthropic/claude-sonnet-4-6", "anthropic/claude-haiku-4-5"]}
        )
        mock_enrich.side_effect = _mock_enrich

        config = Settings(
            providers=[
                ProviderConfig(
                    prefix="anthropic",
                    api_base="https://proxy.example.com/anthropic",
                    api_key="sk-proxy",
                )
            ]
        )
        registry, _ = await build_registry(config)

        assert len(registry.get_all()) == 2
        assert registry.get_by_name("anthropic/claude-sonnet-4-6") is not None
        model = registry.get_by_name("anthropic/claude-sonnet-4-6")
        assert model.api_base == "https://proxy.example.com/anthropic"
        assert model.api_key == "sk-proxy"

    @pytest.mark.asyncio
    @patch("app.discovery.registry_builder.detect_providers")
    @patch("app.discovery.registry_builder.list_models_for_provider")
    @patch("app.discovery.registry_builder.enrich_model")
    async def test_config_provider_overrides_auto_discovery_by_prefix(
        self, mock_enrich, mock_list, mock_detect
    ):
        mock_detect.return_value = [
            DetectedProvider(prefix="anthropic", api_key="sk-direct"),
        ]
        mock_list.side_effect = _mock_list_models({"anthropic": ["anthropic/claude-sonnet-4-6"]})
        mock_enrich.side_effect = _mock_enrich

        config = Settings(
            providers=[
                ProviderConfig(
                    prefix="anthropic",
                    api_base="https://proxy.example.com",
                    api_key="sk-proxy",
                )
            ]
        )
        registry, _ = await build_registry(config)

        model = registry.get_by_name("anthropic/claude-sonnet-4-6")
        assert model.api_key == "sk-proxy"
        assert model.api_base == "https://proxy.example.com"

    @pytest.mark.asyncio
    @patch("app.discovery.registry_builder.detect_providers")
    @patch("app.discovery.registry_builder.list_models_for_provider")
    @patch("app.discovery.registry_builder.enrich_model")
    async def test_config_provider_merges_with_local_discovery(
        self, mock_enrich, mock_list, mock_detect
    ):
        mock_detect.return_value = [
            DetectedProvider(
                prefix="ollama",
                api_base="http://localhost:11434",
                is_local=True,
            ),
        ]
        mock_list.side_effect = _mock_list_models(
            {
                "anthropic": ["anthropic/claude-sonnet-4-6"],
                "ollama": ["ollama/llama3"],
            }
        )
        mock_enrich.side_effect = _mock_enrich

        config = Settings(
            providers=[
                ProviderConfig(
                    prefix="anthropic",
                    api_base="https://proxy.example.com",
                    api_key="sk-proxy",
                )
            ]
        )
        registry, _ = await build_registry(config)

        assert len(registry.get_all()) == 2
        cloud = registry.get_by_name("anthropic/claude-sonnet-4-6")
        assert cloud.api_base == "https://proxy.example.com"
        local = registry.get_by_name("ollama/llama3")
        assert local.is_local is True

    @pytest.mark.asyncio
    @patch("app.discovery.registry_builder.detect_providers")
    @patch("app.discovery.registry_builder.list_models_for_provider")
    @patch("app.discovery.registry_builder.enrich_model")
    async def test_config_provider_with_api_key_env(
        self, mock_enrich, mock_list, mock_detect, monkeypatch
    ):
        monkeypatch.setenv("MY_PROXY_TOKEN", "sk-from-env")
        mock_detect.return_value = []
        mock_list.side_effect = _mock_list_models({"anthropic": ["anthropic/claude-sonnet-4-6"]})
        mock_enrich.side_effect = _mock_enrich

        config = Settings(
            providers=[
                ProviderConfig(
                    prefix="anthropic",
                    api_base="https://proxy.example.com",
                    api_key_env="MY_PROXY_TOKEN",
                )
            ]
        )
        registry, _ = await build_registry(config)

        model = registry.get_by_name("anthropic/claude-sonnet-4-6")
        assert model.api_key == "sk-from-env"

    @pytest.mark.asyncio
    @patch("app.discovery.registry_builder.detect_providers")
    @patch("app.discovery.registry_builder.list_models_for_provider")
    async def test_config_provider_skipped_when_env_var_missing(self, mock_list, mock_detect):
        mock_detect.return_value = []
        mock_list.return_value = []

        config = Settings(
            providers=[
                ProviderConfig(
                    prefix="anthropic",
                    api_base="https://proxy.example.com",
                    api_key_env="MISSING_VAR",
                )
            ]
        )

        with pytest.raises(SystemExit):
            await build_registry(config)

        mock_list.assert_not_called()


class TestResolveConfigProviders:
    def test_resolves_direct_api_key(self):
        providers = _resolve_config_providers(
            [
                ProviderConfig(
                    prefix="anthropic", api_base="https://proxy.example.com", api_key="sk-direct"
                )
            ]
        )
        assert len(providers) == 1
        assert providers[0].prefix == "anthropic"
        assert providers[0].api_key == "sk-direct"
        assert providers[0].api_base == "https://proxy.example.com"

    def test_resolves_api_key_from_env(self, monkeypatch):
        monkeypatch.setenv("MY_TOKEN", "sk-env-value")
        providers = _resolve_config_providers(
            [
                ProviderConfig(
                    prefix="anthropic", api_base="https://proxy.example.com", api_key_env="MY_TOKEN"
                )
            ]
        )
        assert len(providers) == 1
        assert providers[0].api_key == "sk-env-value"

    def test_skips_provider_when_env_var_missing(self):
        providers = _resolve_config_providers(
            [
                ProviderConfig(
                    prefix="anthropic", api_base="https://proxy.example.com", api_key_env="NOT_SET"
                )
            ]
        )
        assert len(providers) == 0

    def test_direct_api_key_takes_precedence_over_env(self, monkeypatch):
        monkeypatch.setenv("MY_TOKEN", "sk-from-env")
        providers = _resolve_config_providers(
            [
                ProviderConfig(
                    prefix="anthropic",
                    api_base="https://proxy.example.com",
                    api_key="sk-direct",
                    api_key_env="MY_TOKEN",
                )
            ]
        )
        assert len(providers) == 1
        assert providers[0].api_key == "sk-direct"

    def test_resolves_multiple_providers(self, monkeypatch):
        monkeypatch.setenv("ANTHROPIC_TOKEN", "sk-anthropic")
        providers = _resolve_config_providers(
            [
                ProviderConfig(
                    prefix="anthropic",
                    api_base="https://proxy.example.com/anthropic",
                    api_key_env="ANTHROPIC_TOKEN",
                ),
                ProviderConfig(
                    prefix="openai",
                    api_base="https://proxy.example.com/openai",
                    api_key="sk-openai",
                ),
            ]
        )
        assert len(providers) == 2
        assert providers[0].prefix == "anthropic"
        assert providers[1].prefix == "openai"

    def test_provider_with_no_auth(self):
        providers = _resolve_config_providers(
            [ProviderConfig(prefix="ollama", api_base="http://remote-ollama:11434")]
        )
        assert len(providers) == 1
        assert providers[0].api_key is None

    def test_resolves_api_base_from_env(self, monkeypatch):
        monkeypatch.setenv("PROXY_URL", "https://proxy.example.com/anthropic")
        providers = _resolve_config_providers(
            [
                ProviderConfig(
                    prefix="anthropic",
                    api_base_env="PROXY_URL",
                    api_key="sk-test",
                )
            ]
        )
        assert len(providers) == 1
        assert providers[0].api_base == "https://proxy.example.com/anthropic"

    def test_direct_api_base_takes_precedence_over_env(self, monkeypatch):
        monkeypatch.setenv("PROXY_URL", "https://env-proxy.example.com")
        providers = _resolve_config_providers(
            [
                ProviderConfig(
                    prefix="anthropic",
                    api_base="https://direct-proxy.example.com",
                    api_base_env="PROXY_URL",
                    api_key="sk-test",
                )
            ]
        )
        assert len(providers) == 1
        assert providers[0].api_base == "https://direct-proxy.example.com"

    def test_skips_provider_when_api_base_env_missing(self):
        providers = _resolve_config_providers(
            [
                ProviderConfig(
                    prefix="anthropic",
                    api_base_env="MISSING_PROXY_URL",
                    api_key="sk-test",
                )
            ]
        )
        assert len(providers) == 0

    def test_skips_provider_when_no_api_base_or_env(self):
        providers = _resolve_config_providers(
            [
                ProviderConfig(
                    prefix="anthropic",
                    api_key="sk-test",
                )
            ]
        )
        assert len(providers) == 0

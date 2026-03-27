from __future__ import annotations

from unittest.mock import AsyncMock, patch

import pytest

from app.config import ModelConfig, Settings
from app.discovery.providers import DetectedProvider
from app.discovery.registry_builder import build_registry


def _mock_detect_providers(providers):
    return AsyncMock(return_value=providers)


def _mock_list_models(model_map):
    async def _list(provider):
        return model_map.get(provider.prefix, [])

    return _list


def _mock_enrich(name, provider):
    return ModelConfig(
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

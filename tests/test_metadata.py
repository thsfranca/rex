from __future__ import annotations

from unittest.mock import patch

from app.discovery.metadata import enrich_model
from app.discovery.providers import DetectedProvider


class TestEnrichModel:
    def test_enriches_with_litellm_cost(self):
        provider = DetectedProvider(prefix="openai", api_key="sk-test")

        with patch("app.discovery.metadata.litellm") as mock_litellm:
            mock_litellm.get_model_info.return_value = {
                "input_cost_per_token": 0.000005,
                "output_cost_per_token": 0.000015,
                "max_tokens": 128000,
                "supports_function_calling": True,
                "supports_reasoning": True,
                "supports_vision": True,
            }
            model = enrich_model("openai/gpt-4o", provider)

        assert model.name == "openai/gpt-4o"
        assert model.api_key == "sk-test"
        assert model.cost_per_1k_input == 0.005
        assert model.is_local is False
        assert model.max_context_window == 128000
        assert model.supports_function_calling is True
        assert model.supports_reasoning is True
        assert model.supports_vision is True

    def test_defaults_when_litellm_has_no_info(self):
        provider = DetectedProvider(prefix="openai", api_key="sk-test")

        with patch("app.discovery.metadata.litellm") as mock_litellm:
            mock_litellm.get_model_info.side_effect = Exception("not found")
            model = enrich_model("openai/custom-model", provider)

        assert model.name == "openai/custom-model"
        assert model.cost_per_1k_input == 0.0
        assert model.is_local is False
        assert model.max_context_window is None
        assert model.supports_function_calling is False
        assert model.supports_reasoning is False
        assert model.supports_vision is False

    def test_local_model_keeps_is_local(self):
        provider = DetectedProvider(
            prefix="ollama",
            api_base="http://localhost:11434",
            is_local=True,
        )

        with patch("app.discovery.metadata.litellm") as mock_litellm:
            mock_litellm.get_model_info.side_effect = Exception("not found")
            model = enrich_model("ollama/llama3", provider)

        assert model.name == "ollama/llama3"
        assert model.is_local is True
        assert model.api_base == "http://localhost:11434"
        assert model.cost_per_1k_input == 0.0

    def test_passes_api_base_through(self):
        provider = DetectedProvider(
            prefix="ollama",
            api_base="http://192.168.1.100:11434",
            is_local=True,
        )

        with patch("app.discovery.metadata.litellm") as mock_litellm:
            mock_litellm.get_model_info.side_effect = Exception("not found")
            model = enrich_model("ollama/llama3", provider)

        assert model.api_base == "http://192.168.1.100:11434"

    def test_handles_none_cost_from_litellm(self):
        provider = DetectedProvider(prefix="openai", api_key="sk-test")

        with patch("app.discovery.metadata.litellm") as mock_litellm:
            mock_litellm.get_model_info.return_value = {
                "input_cost_per_token": None,
            }
            model = enrich_model("openai/gpt-4o", provider)

        assert model.cost_per_1k_input == 0.0

    def test_prefers_max_input_tokens_over_max_tokens(self):
        provider = DetectedProvider(prefix="openai", api_key="sk-test")

        with patch("app.discovery.metadata.litellm") as mock_litellm:
            mock_litellm.get_model_info.return_value = {
                "input_cost_per_token": 0.000001,
                "max_input_tokens": 200000,
                "max_tokens": 128000,
                "supports_function_calling": False,
                "supports_vision": False,
            }
            model = enrich_model("openai/gpt-4o", provider)

        assert model.max_context_window == 200000

    def test_falls_back_to_max_tokens_when_no_max_input_tokens(self):
        provider = DetectedProvider(prefix="openai", api_key="sk-test")

        with patch("app.discovery.metadata.litellm") as mock_litellm:
            mock_litellm.get_model_info.return_value = {
                "input_cost_per_token": 0.000001,
                "max_tokens": 64000,
                "supports_function_calling": True,
                "supports_reasoning": False,
                "supports_vision": False,
            }
            model = enrich_model("openai/gpt-4o", provider)

        assert model.max_context_window == 64000
        assert model.supports_function_calling is True
        assert model.supports_reasoning is False
        assert model.supports_vision is False

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
            }
            model = enrich_model("openai/gpt-4o", provider)

        assert model.name == "openai/gpt-4o"
        assert model.api_key == "sk-test"
        assert model.cost_per_1k_input == 0.005
        assert model.is_local is False

    def test_defaults_when_litellm_has_no_info(self):
        provider = DetectedProvider(prefix="openai", api_key="sk-test")

        with patch("app.discovery.metadata.litellm") as mock_litellm:
            mock_litellm.get_model_info.side_effect = Exception("not found")
            model = enrich_model("openai/custom-model", provider)

        assert model.name == "openai/custom-model"
        assert model.cost_per_1k_input == 0.0
        assert model.is_local is False

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

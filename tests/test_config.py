from __future__ import annotations

import pytest
import yaml

from app.config import ModelConfig, RoutingConfig, Settings, load_config


VALID_MODELS = [
    {
        "name": "openai/gpt-4o",
        "provider": "openai",
        "context_window": 128000,
        "cost_per_1k_input": 0.005,
        "cost_per_1k_output": 0.015,
        "strengths": ["debugging", "generation"],
        "max_latency_ms": 2000,
        "is_local": False,
    },
    {
        "name": "ollama/llama3",
        "provider": "ollama",
        "context_window": 8192,
        "cost_per_1k_input": 0,
        "cost_per_1k_output": 0,
        "strengths": ["completion"],
        "max_latency_ms": 100,
        "is_local": True,
        "api_base": "http://localhost:11434",
    },
]


def _valid_config() -> dict:
    return {
        "models": VALID_MODELS,
        "routing": {
            "completion_model": "ollama/llama3",
            "default_model": "openai/gpt-4o",
        },
    }


class TestModelConfig:
    def test_required_fields(self):
        model = ModelConfig(**VALID_MODELS[0])
        assert model.name == "openai/gpt-4o"
        assert model.provider == "openai"
        assert model.context_window == 128000
        assert model.is_local is False

    def test_optional_fields_default_to_none(self):
        model = ModelConfig(**VALID_MODELS[0])
        assert model.api_key is None
        assert model.api_base is None

    def test_optional_fields_set(self):
        model = ModelConfig(**VALID_MODELS[1])
        assert model.api_base == "http://localhost:11434"


class TestSettings:
    def test_valid_config(self):
        settings = Settings(**_valid_config())
        assert len(settings.models) == 2
        assert settings.routing.completion_model == "ollama/llama3"
        assert settings.routing.default_model == "openai/gpt-4o"

    def test_default_server_config(self):
        settings = Settings(**_valid_config())
        assert settings.server.host == "0.0.0.0"
        assert settings.server.port == 8000

    def test_custom_server_config(self):
        config = _valid_config()
        config["server"] = {"host": "127.0.0.1", "port": 9000}
        settings = Settings(**config)
        assert settings.server.host == "127.0.0.1"
        assert settings.server.port == 9000

    def test_routing_references_invalid_completion_model(self):
        config = _valid_config()
        config["routing"]["completion_model"] = "nonexistent/model"
        with pytest.raises(ValueError, match="nonexistent/model"):
            Settings(**config)

    def test_routing_references_invalid_default_model(self):
        config = _valid_config()
        config["routing"]["default_model"] = "nonexistent/model"
        with pytest.raises(ValueError, match="nonexistent/model"):
            Settings(**config)

    def test_missing_models_field(self):
        with pytest.raises(Exception):
            Settings(routing=RoutingConfig(completion_model="a", default_model="b"))

    def test_missing_routing_field(self):
        with pytest.raises(Exception):
            Settings(models=[ModelConfig(**VALID_MODELS[0])])


class TestLoadConfig:
    def test_load_valid_yaml(self, tmp_path):
        config_file = tmp_path / "config.yaml"
        config_file.write_text(yaml.dump(_valid_config()))
        settings = load_config(config_file)
        assert settings.routing.default_model == "openai/gpt-4o"

    def test_load_missing_file(self, tmp_path):
        with pytest.raises(SystemExit):
            load_config(tmp_path / "missing.yaml")

    def test_load_invalid_yaml(self, tmp_path):
        config_file = tmp_path / "config.yaml"
        config_file.write_text("models: not_a_list")
        with pytest.raises(Exception):
            load_config(config_file)

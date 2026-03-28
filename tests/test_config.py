from __future__ import annotations

from pathlib import Path

import yaml

from app.config import (
    DEFAULT_CONFIG_PATH,
    EnrichmentsConfig,
    LLMJudgeConfig,
    ModelConfig,
    ProviderConfig,
    RoutingConfig,
    ServerConfig,
    Settings,
    load_config,
)


class TestServerConfig:
    def test_defaults(self):
        server = ServerConfig()
        assert server.host == "0.0.0.0"
        assert server.port == 8000
        assert server.timeout == 600
        assert server.stream_timeout == 600

    def test_custom_timeout(self):
        server = ServerConfig(timeout=300)
        assert server.timeout == 300

    def test_custom_stream_timeout(self):
        server = ServerConfig(stream_timeout=120)
        assert server.stream_timeout == 120


class TestModelConfig:
    def test_only_name_required(self):
        model = ModelConfig(name="openai/gpt-4o")
        assert model.name == "openai/gpt-4o"
        assert model.api_key is None
        assert model.api_base is None
        assert model.cost_per_1k_input == 0.0
        assert model.is_local is False
        assert model.max_context_window is None
        assert model.supports_function_calling is False
        assert model.supports_reasoning is False
        assert model.supports_vision is False
        assert model.timeout is None

    def test_all_fields_set(self):
        model = ModelConfig(
            name="ollama/llama3",
            api_key="sk-test",
            api_base="http://localhost:11434",
            cost_per_1k_input=0.001,
            is_local=True,
            max_context_window=128000,
            supports_function_calling=True,
            supports_reasoning=True,
            supports_vision=True,
            timeout=60,
        )
        assert model.name == "ollama/llama3"
        assert model.api_key == "sk-test"
        assert model.api_base == "http://localhost:11434"
        assert model.cost_per_1k_input == 0.001
        assert model.is_local is True
        assert model.max_context_window == 128000
        assert model.supports_function_calling is True
        assert model.supports_reasoning is True
        assert model.supports_vision is True
        assert model.timeout == 60

    def test_timeout_defaults_to_none(self):
        model = ModelConfig(name="test/model")
        assert model.timeout is None

    def test_timeout_can_be_set(self):
        model = ModelConfig(name="test/model", timeout=30)
        assert model.timeout == 30


class TestProviderConfig:
    def test_requires_prefix_and_api_base(self):
        provider = ProviderConfig(prefix="anthropic", api_base="https://proxy.example.com")
        assert provider.prefix == "anthropic"
        assert provider.api_base == "https://proxy.example.com"
        assert provider.api_base_env is None
        assert provider.api_key is None
        assert provider.api_key_env is None

    def test_with_direct_api_key(self):
        provider = ProviderConfig(
            prefix="anthropic",
            api_base="https://proxy.example.com",
            api_key="sk-test",
        )
        assert provider.api_key == "sk-test"
        assert provider.api_key_env is None

    def test_with_api_key_env(self):
        provider = ProviderConfig(
            prefix="anthropic",
            api_base="https://proxy.example.com",
            api_key_env="ANTHROPIC_AUTH_TOKEN",
        )
        assert provider.api_key is None
        assert provider.api_key_env == "ANTHROPIC_AUTH_TOKEN"

    def test_with_api_base_env(self):
        provider = ProviderConfig(
            prefix="anthropic",
            api_base_env="LITELLM_PROXY_URL",
            api_key="sk-test",
        )
        assert provider.api_base is None
        assert provider.api_base_env == "LITELLM_PROXY_URL"

    def test_all_fields_set(self):
        provider = ProviderConfig(
            prefix="openai",
            api_base="https://proxy.example.com/openai",
            api_base_env="PROXY_URL",
            api_key="sk-direct",
            api_key_env="OPENAI_PROXY_KEY",
        )
        assert provider.prefix == "openai"
        assert provider.api_base == "https://proxy.example.com/openai"
        assert provider.api_base_env == "PROXY_URL"
        assert provider.api_key == "sk-direct"
        assert provider.api_key_env == "OPENAI_PROXY_KEY"


class TestRoutingConfig:
    def test_defaults_to_none(self):
        routing = RoutingConfig()
        assert routing.primary_model is None

    def test_explicit_primary_model(self):
        routing = RoutingConfig(primary_model="openai/gpt-4o")
        assert routing.primary_model == "openai/gpt-4o"


class TestEnrichmentsConfig:
    def test_defaults_to_disabled(self):
        enrichments = EnrichmentsConfig()
        assert enrichments.task_decomposition is False

    def test_enable_task_decomposition(self):
        enrichments = EnrichmentsConfig(task_decomposition=True)
        assert enrichments.task_decomposition is True


class TestLLMJudgeConfig:
    def test_defaults_to_disabled(self):
        judge = LLMJudgeConfig()
        assert judge.enabled is False
        assert judge.model is None
        assert judge.confidence_threshold == 0.5

    def test_enable_with_model(self):
        judge = LLMJudgeConfig(enabled=True, model="ollama/llama3")
        assert judge.enabled is True
        assert judge.model == "ollama/llama3"

    def test_custom_threshold(self):
        judge = LLMJudgeConfig(confidence_threshold=0.3)
        assert judge.confidence_threshold == 0.3


class TestSettings:
    def test_all_defaults(self):
        settings = Settings()
        assert settings.server.host == "0.0.0.0"
        assert settings.server.port == 8000
        assert settings.server.timeout == 600
        assert settings.server.stream_timeout == 600
        assert settings.models == []
        assert settings.providers == []
        assert settings.routing.primary_model is None
        assert settings.enrichments.task_decomposition is False
        assert settings.llm_judge.enabled is False

    def test_custom_server_config(self):
        settings = Settings(server={"host": "127.0.0.1", "port": 9000})
        assert settings.server.host == "127.0.0.1"
        assert settings.server.port == 9000

    def test_custom_server_timeouts(self):
        settings = Settings(server={"timeout": 300, "stream_timeout": 120})
        assert settings.server.timeout == 300
        assert settings.server.stream_timeout == 120

    def test_with_models(self):
        settings = Settings(models=[{"name": "openai/gpt-4o"}, {"name": "ollama/llama3"}])
        assert len(settings.models) == 2
        assert settings.models[0].name == "openai/gpt-4o"

    def test_with_routing_override(self):
        settings = Settings(routing={"primary_model": "openai/gpt-4o"})
        assert settings.routing.primary_model == "openai/gpt-4o"

    def test_with_enrichments(self):
        settings = Settings(enrichments={"task_decomposition": True})
        assert settings.enrichments.task_decomposition is True

    def test_enrichments_omitted_defaults_to_disabled(self):
        settings = Settings(models=[{"name": "openai/gpt-4o"}])
        assert settings.enrichments.task_decomposition is False

    def test_with_llm_judge(self):
        settings = Settings(
            llm_judge={"enabled": True, "model": "ollama/llama3", "confidence_threshold": 0.4}
        )
        assert settings.llm_judge.enabled is True
        assert settings.llm_judge.model == "ollama/llama3"
        assert settings.llm_judge.confidence_threshold == 0.4

    def test_llm_judge_omitted_defaults_to_disabled(self):
        settings = Settings(models=[{"name": "openai/gpt-4o"}])
        assert settings.llm_judge.enabled is False
        assert settings.llm_judge.model is None

    def test_with_providers(self):
        settings = Settings(
            providers=[
                {"prefix": "anthropic", "api_base": "https://proxy.example.com"},
                {"prefix": "openai", "api_base": "https://proxy.example.com/openai"},
            ]
        )
        assert len(settings.providers) == 2
        assert settings.providers[0].prefix == "anthropic"
        assert settings.providers[1].prefix == "openai"

    def test_providers_omitted_defaults_to_empty(self):
        settings = Settings(models=[{"name": "openai/gpt-4o"}])
        assert settings.providers == []


class TestDefaultConfigPath:
    def test_points_to_home_rex_directory(self):
        assert DEFAULT_CONFIG_PATH == Path.home() / ".rex" / "config.yaml"


class TestLoadConfig:
    def test_returns_none_when_file_missing(self, tmp_path):
        result = load_config(tmp_path / "missing.yaml")
        assert result is None

    def test_loads_valid_yaml(self, tmp_path):
        config_file = tmp_path / "config.yaml"
        config_file.write_text(
            yaml.dump(
                {
                    "models": [{"name": "openai/gpt-4o"}],
                    "routing": {"primary_model": "openai/gpt-4o"},
                }
            )
        )
        settings = load_config(config_file)
        assert settings is not None
        assert settings.routing.primary_model == "openai/gpt-4o"
        assert len(settings.models) == 1

    def test_loads_empty_yaml(self, tmp_path):
        config_file = tmp_path / "config.yaml"
        config_file.write_text("")
        settings = load_config(config_file)
        assert settings is not None
        assert settings.models == []

    def test_loads_minimal_yaml(self, tmp_path):
        config_file = tmp_path / "config.yaml"
        config_file.write_text(yaml.dump({"server": {"port": 9000}}))
        settings = load_config(config_file)
        assert settings is not None
        assert settings.server.port == 9000
        assert settings.models == []

    def test_loads_enrichments_from_yaml(self, tmp_path):
        config_file = tmp_path / "config.yaml"
        config_file.write_text(yaml.dump({"enrichments": {"task_decomposition": True}}))
        settings = load_config(config_file)
        assert settings is not None
        assert settings.enrichments.task_decomposition is True

    def test_loads_llm_judge_from_yaml(self, tmp_path):
        config_file = tmp_path / "config.yaml"
        config_file.write_text(
            yaml.dump({"llm_judge": {"enabled": True, "model": "ollama/llama3"}})
        )
        settings = load_config(config_file)
        assert settings is not None
        assert settings.llm_judge.enabled is True
        assert settings.llm_judge.model == "ollama/llama3"
        assert settings.llm_judge.confidence_threshold == 0.5

    def test_loads_learning_config_from_yaml(self, tmp_path):
        config_file = tmp_path / "config.yaml"
        config_file.write_text(
            yaml.dump({"learning": {"db_path": "/tmp/test.db", "embeddings_model": "custom-model"}})
        )
        settings = load_config(config_file)
        assert settings is not None
        assert settings.learning.db_path == "/tmp/test.db"
        assert settings.learning.embeddings_model == "custom-model"

    def test_learning_config_defaults(self):
        settings = Settings()
        assert settings.learning.db_path == "~/.rex/decisions.db"
        assert settings.learning.embeddings_model == "all-MiniLM-L6-v2"
        assert settings.learning.recluster_interval == 100
        assert settings.learning.max_k == 20
        assert settings.learning.promotion_silhouette_threshold == 0.5

    def test_loads_learning_pipeline_config_from_yaml(self, tmp_path):
        config_file = tmp_path / "config.yaml"
        config_file.write_text(
            yaml.dump(
                {
                    "learning": {
                        "recluster_interval": 200,
                        "max_k": 30,
                        "promotion_silhouette_threshold": 0.6,
                    }
                }
            )
        )
        settings = load_config(config_file)
        assert settings is not None
        assert settings.learning.recluster_interval == 200
        assert settings.learning.max_k == 30
        assert settings.learning.promotion_silhouette_threshold == 0.6

    def test_loads_providers_from_yaml(self, tmp_path):
        config_file = tmp_path / "config.yaml"
        config_file.write_text(
            yaml.dump(
                {
                    "providers": [
                        {
                            "prefix": "anthropic",
                            "api_base": "https://proxy.example.com/anthropic",
                            "api_key_env": "ANTHROPIC_AUTH_TOKEN",
                        }
                    ]
                }
            )
        )
        settings = load_config(config_file)
        assert settings is not None
        assert len(settings.providers) == 1
        assert settings.providers[0].prefix == "anthropic"
        assert settings.providers[0].api_base == "https://proxy.example.com/anthropic"
        assert settings.providers[0].api_key_env == "ANTHROPIC_AUTH_TOKEN"

    def test_loads_providers_with_direct_api_key_from_yaml(self, tmp_path):
        config_file = tmp_path / "config.yaml"
        config_file.write_text(
            yaml.dump(
                {
                    "providers": [
                        {
                            "prefix": "openai",
                            "api_base": "https://proxy.example.com/openai",
                            "api_key": "sk-proxy-key",
                        }
                    ]
                }
            )
        )
        settings = load_config(config_file)
        assert settings is not None
        assert len(settings.providers) == 1
        assert settings.providers[0].api_key == "sk-proxy-key"
        assert settings.providers[0].api_key_env is None

    def test_loads_server_timeouts_from_yaml(self, tmp_path):
        config_file = tmp_path / "config.yaml"
        config_file.write_text(yaml.dump({"server": {"timeout": 300, "stream_timeout": 120}}))
        settings = load_config(config_file)
        assert settings is not None
        assert settings.server.timeout == 300
        assert settings.server.stream_timeout == 120

    def test_loads_model_timeout_from_yaml(self, tmp_path):
        config_file = tmp_path / "config.yaml"
        config_file.write_text(
            yaml.dump(
                {
                    "models": [
                        {"name": "ollama/llama3", "timeout": 60},
                        {"name": "openai/gpt-4o"},
                    ]
                }
            )
        )
        settings = load_config(config_file)
        assert settings is not None
        assert settings.models[0].timeout == 60
        assert settings.models[1].timeout is None

    def test_load_invalid_yaml(self, tmp_path):
        config_file = tmp_path / "config.yaml"
        config_file.write_text("models: not_a_list")
        try:
            load_config(config_file)
            assert False, "Should have raised"
        except Exception:
            pass

from __future__ import annotations

import yaml

from app.config import (
    EnrichmentsConfig,
    LLMJudgeConfig,
    ModelConfig,
    RoutingConfig,
    Settings,
    load_config,
)


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
        assert settings.models == []
        assert settings.routing.primary_model is None
        assert settings.enrichments.task_decomposition is False
        assert settings.llm_judge.enabled is False

    def test_custom_server_config(self):
        settings = Settings(server={"host": "127.0.0.1", "port": 9000})
        assert settings.server.host == "127.0.0.1"
        assert settings.server.port == 9000

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

    def test_load_invalid_yaml(self, tmp_path):
        config_file = tmp_path / "config.yaml"
        config_file.write_text("models: not_a_list")
        try:
            load_config(config_file)
            assert False, "Should have raised"
        except Exception:
            pass

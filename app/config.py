from __future__ import annotations

from pathlib import Path

import yaml
from pydantic import BaseModel

DEFAULT_CONFIG_PATH = Path("~/.rex/config.yaml").expanduser()


class ServerConfig(BaseModel):
    host: str = "0.0.0.0"
    port: int = 8000
    timeout: float = 600
    stream_timeout: float = 600


class ModelConfig(BaseModel):
    name: str
    api_key: str | None = None
    api_base: str | None = None
    cost_per_1k_input: float = 0.0
    is_local: bool = False
    max_context_window: int | None = None
    supports_function_calling: bool = False
    supports_reasoning: bool = False
    supports_vision: bool = False
    timeout: float | None = None


class ProviderConfig(BaseModel):
    prefix: str
    api_base: str | None = None
    api_base_env: str | None = None
    api_key: str | None = None
    api_key_env: str | None = None


class RoutingConfig(BaseModel):
    primary_model: str | None = None
    chat_model: str | None = None


class EnrichmentsConfig(BaseModel):
    task_decomposition: bool = False


class LLMJudgeConfig(BaseModel):
    enabled: bool = False
    model: str | None = None
    confidence_threshold: float = 0.5


class LearningConfig(BaseModel):
    db_path: str = "~/.rex/decisions.db"
    embeddings_model: str = "all-MiniLM-L6-v2"
    recluster_interval: int = 100
    max_k: int = 20
    promotion_silhouette_threshold: float = 0.5


class Settings(BaseModel):
    server: ServerConfig = ServerConfig()
    models: list[ModelConfig] = []
    providers: list[ProviderConfig] = []
    routing: RoutingConfig = RoutingConfig()
    enrichments: EnrichmentsConfig = EnrichmentsConfig()
    llm_judge: LLMJudgeConfig = LLMJudgeConfig()
    learning: LearningConfig = LearningConfig()


def load_config(path: str | Path | None = None) -> Settings | None:
    config_path = Path(path) if path is not None else DEFAULT_CONFIG_PATH
    if not config_path.exists():
        return None

    with open(config_path) as f:
        raw = yaml.safe_load(f)

    if raw is None:
        return Settings()

    return Settings(**raw)

from __future__ import annotations

from pathlib import Path

import yaml
from pydantic import BaseModel


class ServerConfig(BaseModel):
    host: str = "0.0.0.0"
    port: int = 8000


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


class RoutingConfig(BaseModel):
    primary_model: str | None = None


class EnrichmentsConfig(BaseModel):
    task_decomposition: bool = False


class Settings(BaseModel):
    server: ServerConfig = ServerConfig()
    models: list[ModelConfig] = []
    routing: RoutingConfig = RoutingConfig()
    enrichments: EnrichmentsConfig = EnrichmentsConfig()


def load_config(path: str | Path) -> Settings | None:
    config_path = Path(path)
    if not config_path.exists():
        return None

    with open(config_path) as f:
        raw = yaml.safe_load(f)

    if raw is None:
        return Settings()

    return Settings(**raw)

from __future__ import annotations

import sys
from pathlib import Path

import yaml
from pydantic import BaseModel, model_validator


class ServerConfig(BaseModel):
    host: str = "0.0.0.0"
    port: int = 8000


class ModelConfig(BaseModel):
    name: str
    provider: str
    context_window: int
    cost_per_1k_input: float
    cost_per_1k_output: float
    strengths: list[str]
    max_latency_ms: int
    is_local: bool
    api_key: str | None = None
    api_base: str | None = None


class RoutingConfig(BaseModel):
    completion_model: str
    default_model: str


class Settings(BaseModel):
    server: ServerConfig = ServerConfig()
    models: list[ModelConfig]
    routing: RoutingConfig

    @model_validator(mode="after")
    def validate_routing_references(self) -> Settings:
        model_names = {m.name for m in self.models}
        for field in ("completion_model", "default_model"):
            ref = getattr(self.routing, field)
            if ref not in model_names:
                raise ValueError(
                    f"Model '{ref}' referenced in routing.{field} "
                    f"but not defined in models"
                )
        return self


def load_config(path: str | Path) -> Settings:
    config_path = Path(path)
    if not config_path.exists():
        print(f"Config file not found: {config_path}", file=sys.stderr)
        sys.exit(1)

    with open(config_path) as f:
        raw = yaml.safe_load(f)

    return Settings(**raw)

from __future__ import annotations

import logging

from app.config import ModelConfig
from app.router.detector import detect_feature
from app.router.registry import ModelRegistry

logger = logging.getLogger(__name__)


class RoutingEngine:
    def __init__(self, registry: ModelRegistry, primary_model: str | None = None) -> None:
        self._registry = registry
        self._primary = self._resolve_primary(primary_model)

    def _resolve_primary(self, override: str | None) -> ModelConfig:
        if override:
            model = self._registry.get_by_name(override)
            if model is None:
                raise ValueError(f"Primary model '{override}' not found in registry")
            return model

        by_cost = self._registry.sorted_by_cost()
        if not by_cost:
            raise ValueError("No models available in registry")
        return by_cost[0]

    @property
    def primary(self) -> ModelConfig:
        return self._primary

    def select_model(
        self,
        messages: list[dict],
        max_tokens: int | None = None,
        temperature: float | None = None,
    ) -> ModelConfig:
        detect_feature(messages, max_tokens, temperature)
        return self._primary

    def fallback_order(self, primary: ModelConfig) -> list[ModelConfig]:
        by_cost = self._registry.sorted_by_cost()
        return [m for m in by_cost if m.name != primary.name]

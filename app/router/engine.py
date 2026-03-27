from __future__ import annotations

import logging

from app.config import ModelConfig, RoutingConfig
from app.router.detector import FeatureType, detect_feature
from app.router.registry import ModelRegistry

logger = logging.getLogger(__name__)


class RoutingEngine:
    def __init__(self, registry: ModelRegistry, routing_config: RoutingConfig) -> None:
        self._registry = registry
        self._routing_config = routing_config

    def select_model(
        self,
        messages: list[dict],
        max_tokens: int | None = None,
        temperature: float | None = None,
    ) -> ModelConfig:
        feature = detect_feature(messages, max_tokens, temperature)
        if feature == FeatureType.COMPLETION:
            model_name = self._routing_config.completion_model
        else:
            model_name = self._routing_config.default_model

        model = self._registry.get_by_name(model_name)
        if model is None:
            raise ValueError(f"Configured model '{model_name}' not found in registry")
        return model

    def fallback_order(self, primary: ModelConfig) -> list[ModelConfig]:
        candidates = []
        default_name = self._routing_config.default_model
        if primary.name != default_name:
            default_model = self._registry.get_by_name(default_name)
            if default_model is not None:
                candidates.append(default_model)

        for model in self._registry.get_all():
            if model.name != primary.name and model not in candidates:
                candidates.append(model)

        return candidates

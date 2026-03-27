from __future__ import annotations

import logging

from app.config import ModelConfig
from app.router.categories import TaskRequirements, get_requirements
from app.router.classifier import classify
from app.router.detector import FeatureType, detect_feature
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

    def _model_meets_requirements(self, model: ModelConfig, requirements: TaskRequirements) -> bool:
        if requirements.min_context_window is not None:
            if (
                model.max_context_window is None
                or model.max_context_window < requirements.min_context_window
            ):
                return False
        if requirements.needs_function_calling:
            if not model.supports_function_calling:
                return False
        if requirements.needs_cloud:
            if model.is_local:
                return False
        return True

    def select_model(
        self,
        messages: list[dict],
        max_tokens: int | None = None,
        temperature: float | None = None,
    ) -> ModelConfig:
        feature = detect_feature(messages, max_tokens, temperature)

        if feature == FeatureType.COMPLETION:
            return self._primary

        result = classify(messages)
        requirements = get_requirements(result.category)

        if self._model_meets_requirements(self._primary, requirements):
            return self._primary

        candidates = self._registry.filter_by_requirements(requirements)
        if candidates:
            logger.info(
                "Task classified as %s (confidence=%.2f), "
                "routing to %s (primary %s does not meet requirements)",
                result.category.value,
                result.confidence,
                candidates[0].name,
                self._primary.name,
            )
            return candidates[0]

        logger.info(
            "Task classified as %s but no model meets requirements, " "falling back to primary",
            result.category.value,
        )
        return self._primary

    def fallback_order(self, primary: ModelConfig) -> list[ModelConfig]:
        by_cost = self._registry.sorted_by_cost()
        return [m for m in by_cost if m.name != primary.name]

from __future__ import annotations

import logging
from dataclasses import dataclass

from app.config import ModelConfig
from app.learning.centroids import CentroidClassifier
from app.router.categories import TaskCategory, get_requirements
from app.router.classifier import ClassificationResult, classify
from app.router.detector import FeatureType, detect_feature
from app.router.llm_judge import LLMJudge
from app.router.ml_classifier import MLClassifier
from app.router.registry import ModelRegistry

logger = logging.getLogger(__name__)


@dataclass(frozen=True)
class RoutingDecision:
    model: ModelConfig
    category: TaskCategory
    confidence: float
    feature_type: FeatureType
    scores: dict[TaskCategory, float] | None = None


class RoutingEngine:
    def __init__(
        self,
        registry: ModelRegistry,
        primary_model: str | None = None,
        judge: LLMJudge | None = None,
        confidence_threshold: float = 0.5,
        centroid_classifier: CentroidClassifier | None = None,
        ml_classifier: MLClassifier | None = None,
        ml_promoted: bool = False,
    ) -> None:
        self._registry = registry
        self._primary = self._resolve_primary(primary_model)
        self._judge = judge
        self._confidence_threshold = confidence_threshold
        self._centroid_classifier = centroid_classifier
        self._ml_classifier = ml_classifier
        self._ml_promoted = ml_promoted

    def set_ml_promoted(self, promoted: bool) -> None:
        self._ml_promoted = promoted

    def set_centroid_classifier(self, classifier: CentroidClassifier | None) -> None:
        self._centroid_classifier = classifier

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

    @property
    def registry(self) -> ModelRegistry:
        return self._registry

    async def select_model(
        self,
        messages: list[dict],
        max_tokens: int | None = None,
        temperature: float | None = None,
        feature_type: FeatureType | None = None,
        embedding=None,
    ) -> RoutingDecision:
        feature = feature_type or detect_feature(messages, max_tokens, temperature)

        if feature == FeatureType.COMPLETION:
            return RoutingDecision(
                model=self._primary,
                category=TaskCategory.COMPLETION,
                confidence=1.0,
                feature_type=feature,
            )

        if (
            self._ml_promoted
            and self._ml_classifier is not None
            and self._ml_classifier.is_trained()
            and embedding is not None
        ):
            result = self._ml_classifier.classify(embedding)
        else:
            result = classify(messages)

        if result.confidence < self._confidence_threshold:
            if self._centroid_classifier is not None and embedding is not None:
                centroid_result = self._centroid_classifier.classify(embedding)
                if centroid_result.confidence >= self._confidence_threshold:
                    logger.info(
                        "Centroid classifier reclassified from %s (confidence=%.2f) "
                        "to %s (confidence=%.2f)",
                        result.category.value,
                        result.confidence,
                        centroid_result.category.value,
                        centroid_result.confidence,
                    )
                    result = centroid_result

            if result.confidence < self._confidence_threshold and self._judge is not None:
                judge_result = await self._judge.classify(messages)
                if judge_result is not None:
                    logger.info(
                        "LLM judge reclassified from %s (confidence=%.2f) to %s",
                        result.category.value,
                        result.confidence,
                        judge_result.category.value,
                    )
                    result = ClassificationResult(
                        category=judge_result.category,
                        confidence=0.9,
                    )

        requirements = get_requirements(result.category)

        if self._registry.meets_requirements(self._primary, requirements):
            return RoutingDecision(
                model=self._primary,
                category=result.category,
                confidence=result.confidence,
                feature_type=feature,
                scores=result.scores if result.scores else None,
            )

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
            return RoutingDecision(
                model=candidates[0],
                category=result.category,
                confidence=result.confidence,
                feature_type=feature,
                scores=result.scores if result.scores else None,
            )

        logger.info(
            "Task classified as %s but no model meets requirements, " "falling back to primary",
            result.category.value,
        )
        return RoutingDecision(
            model=self._primary,
            category=result.category,
            confidence=result.confidence,
            feature_type=feature,
            scores=result.scores if result.scores else None,
        )

    def fallback_order(self, primary: ModelConfig) -> list[ModelConfig]:
        by_cost = self._registry.sorted_by_cost()
        return [m for m in by_cost if m.name != primary.name]

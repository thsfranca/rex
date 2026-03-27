from __future__ import annotations

from dataclasses import dataclass, field

from app.router.categories import TaskCategory
from app.router.detector import FeatureType


@dataclass
class EnrichmentContext:
    messages: list[dict] = field(default_factory=list)
    category: TaskCategory = TaskCategory.GENERAL
    confidence: float = 0.0
    feature_type: FeatureType = FeatureType.CHAT

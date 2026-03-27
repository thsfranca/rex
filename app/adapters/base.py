from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass

from app.router.detector import FeatureType


@dataclass(frozen=True)
class NormalizedRequest:
    messages: list[dict]
    feature_type: FeatureType
    max_tokens: int | None = None
    temperature: float | None = None


class ClientAdapter(ABC):
    @abstractmethod
    def normalize(
        self,
        messages: list[dict],
        max_tokens: int | None = None,
        temperature: float | None = None,
    ) -> NormalizedRequest: ...

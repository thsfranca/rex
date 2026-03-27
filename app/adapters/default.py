from __future__ import annotations

from app.adapters.base import ClientAdapter, NormalizedRequest
from app.router.detector import detect_feature


class DefaultAdapter(ClientAdapter):
    def normalize(
        self,
        messages: list[dict],
        max_tokens: int | None = None,
        temperature: float | None = None,
    ) -> NormalizedRequest:
        feature = detect_feature(messages, max_tokens, temperature)
        return NormalizedRequest(
            messages=messages,
            feature_type=feature,
            max_tokens=max_tokens,
            temperature=temperature,
        )

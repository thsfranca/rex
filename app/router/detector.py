from __future__ import annotations

from enum import Enum


class FeatureType(str, Enum):
    COMPLETION = "completion"
    CHAT = "chat"


_COMPLETION_THRESHOLD = 0.5


def _count_user_messages(messages: list[dict]) -> int:
    return sum(1 for m in messages if m.get("role") == "user")


def _content_length(content) -> int:
    if content is None:
        return 0
    if isinstance(content, str):
        return len(content)
    if isinstance(content, list):
        return sum(len(block.get("text", "") or "") for block in content if isinstance(block, dict))
    return 0


def _estimate_prompt_length(messages: list[dict]) -> int:
    return sum(_content_length(m.get("content")) for m in messages)


def detect_feature(
    messages: list[dict],
    max_tokens: int | None = None,
    temperature: float | None = None,
) -> FeatureType:
    score = 0.0

    user_count = _count_user_messages(messages)
    if user_count <= 1:
        score += 0.3
    else:
        score -= 0.3

    prompt_length = _estimate_prompt_length(messages)
    if prompt_length < 200:
        score += 0.2
    elif prompt_length > 500:
        score -= 0.2

    if max_tokens is not None and max_tokens < 500:
        score += 0.2
    elif max_tokens is None:
        score -= 0.1

    if temperature is not None and temperature <= 0.2:
        score += 0.2
    elif temperature is None or temperature > 0.5:
        score -= 0.1

    if score >= _COMPLETION_THRESHOLD:
        return FeatureType.COMPLETION
    return FeatureType.CHAT

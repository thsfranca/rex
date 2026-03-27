from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime


@dataclass
class DecisionRecord:
    timestamp: datetime
    prompt_hash: str
    category: str
    confidence: float
    feature_type: str
    selected_model: str
    used_model: str
    response_time_ms: int
    input_tokens: int | None = None
    output_tokens: int | None = None
    cost: float | None = None
    fallback_triggered: bool = False
    escalated: bool = False
    rule_votes: dict[str, float] | None = None
    embedding: bytes | None = None

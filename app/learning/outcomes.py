from __future__ import annotations

from dataclasses import dataclass

from app.logging.models import DecisionRecord


@dataclass
class CategoryMetrics:
    fallback_rate: float
    error_rate: float
    average_latency_ms: float
    reask_rate: float


class OutcomeTracker:
    def evaluate(
        self,
        category: str,
        records: list[DecisionRecord],
    ) -> CategoryMetrics | None:
        category_records = [r for r in records if r.category == category]
        if not category_records:
            return None

        n = len(category_records)
        fallback_count = sum(1 for r in category_records if r.fallback_triggered)
        error_count = sum(1 for r in category_records if r.response_time_ms == 0)
        total_latency = sum(r.response_time_ms for r in category_records)

        reask_count = 0
        for i in range(len(category_records) - 1):
            current = category_records[i]
            nxt = category_records[i + 1]
            if current.prompt_hash == nxt.prompt_hash:
                diff = abs((current.timestamp - nxt.timestamp).total_seconds())
                if diff <= 60:
                    reask_count += 1

        return CategoryMetrics(
            fallback_rate=fallback_count / n,
            error_rate=error_count / n,
            average_latency_ms=total_latency / n,
            reask_rate=reask_count / max(n - 1, 1),
        )

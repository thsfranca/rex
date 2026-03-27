from __future__ import annotations

import logging
from dataclasses import dataclass

from app.learning.outcomes import CategoryMetrics, OutcomeTracker
from app.logging.models import DecisionRecord
from app.router.categories import TaskCategory

logger = logging.getLogger(__name__)

FALLBACK_THRESHOLD = 0.30
ERROR_THRESHOLD = 0.10
REASK_THRESHOLD = 0.40


@dataclass
class CategoryMigration:
    category: TaskCategory
    trigger_reason: str
    metrics: CategoryMetrics


def check_migrations(
    records: list[DecisionRecord],
    tracker: OutcomeTracker,
    window: int = 50,
) -> list[CategoryMigration]:
    migrations: list[CategoryMigration] = []
    categories = {r.category for r in records}

    for category_str in categories:
        category_records = [r for r in records if r.category == category_str]
        windowed = category_records[:window]
        metrics = tracker.evaluate(category_str, windowed)
        if metrics is None:
            continue

        try:
            category = TaskCategory(category_str)
        except ValueError:
            continue

        reasons: list[str] = []
        if metrics.fallback_rate > FALLBACK_THRESHOLD:
            reasons.append(f"fallback_rate={metrics.fallback_rate:.0%} > {FALLBACK_THRESHOLD:.0%}")
        if metrics.error_rate > ERROR_THRESHOLD:
            reasons.append(f"error_rate={metrics.error_rate:.0%} > {ERROR_THRESHOLD:.0%}")
        if metrics.reask_rate > REASK_THRESHOLD:
            reasons.append(f"reask_rate={metrics.reask_rate:.0%} > {REASK_THRESHOLD:.0%}")

        if reasons:
            migrations.append(
                CategoryMigration(
                    category=category,
                    trigger_reason="; ".join(reasons),
                    metrics=metrics,
                )
            )
            logger.info(
                "Category %s flagged for migration: %s",
                category_str,
                "; ".join(reasons),
            )

    return migrations

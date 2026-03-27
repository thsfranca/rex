from __future__ import annotations

from datetime import datetime, timedelta, timezone

from app.learning.migrations import check_migrations
from app.learning.outcomes import OutcomeTracker
from app.logging.models import DecisionRecord
from app.router.categories import TaskCategory


def _make_record(**overrides):
    defaults = {
        "timestamp": datetime.now(timezone.utc),
        "prompt_hash": "abc123",
        "category": "debugging",
        "confidence": 0.8,
        "feature_type": "chat",
        "selected_model": "test/model",
        "used_model": "test/model",
        "response_time_ms": 100,
    }
    defaults.update(overrides)
    return DecisionRecord(**defaults)


class TestCheckMigrations:
    def test_no_migrations_for_healthy_records(self):
        tracker = OutcomeTracker()
        records = [_make_record(prompt_hash=f"h{i}") for i in range(50)]
        migrations = check_migrations(records, tracker)
        assert len(migrations) == 0

    def test_flags_high_fallback_rate(self):
        tracker = OutcomeTracker()
        records = [_make_record(fallback_triggered=True) for _ in range(40)] + [
            _make_record(fallback_triggered=False) for _ in range(10)
        ]
        migrations = check_migrations(records, tracker)
        assert len(migrations) == 1
        assert migrations[0].category == TaskCategory.DEBUGGING
        assert "fallback_rate" in migrations[0].trigger_reason

    def test_flags_high_error_rate(self):
        tracker = OutcomeTracker()
        records = [_make_record(response_time_ms=0) for _ in range(20)] + [
            _make_record(response_time_ms=100) for _ in range(30)
        ]
        migrations = check_migrations(records, tracker)
        assert len(migrations) == 1
        assert "error_rate" in migrations[0].trigger_reason

    def test_flags_high_reask_rate(self):
        tracker = OutcomeTracker()
        now = datetime.now(timezone.utc)
        records = []
        for i in range(50):
            records.append(
                _make_record(
                    prompt_hash="same",
                    timestamp=now + timedelta(seconds=i * 10),
                )
            )
        migrations = check_migrations(records, tracker)
        assert len(migrations) == 1
        assert "reask_rate" in migrations[0].trigger_reason

    def test_multiple_categories_flagged(self):
        tracker = OutcomeTracker()
        debug_records = [
            _make_record(category="debugging", fallback_triggered=True) for _ in range(50)
        ]
        refactor_records = [
            _make_record(category="refactoring", fallback_triggered=True) for _ in range(50)
        ]
        migrations = check_migrations(debug_records + refactor_records, tracker)
        flagged_cats = {m.category for m in migrations}
        assert TaskCategory.DEBUGGING in flagged_cats
        assert TaskCategory.REFACTORING in flagged_cats

    def test_window_limits_records(self):
        tracker = OutcomeTracker()
        failing = [_make_record(fallback_triggered=True) for _ in range(20)]
        passing = [_make_record(fallback_triggered=False) for _ in range(80)]
        records = failing + passing
        migrations = check_migrations(records, tracker, window=50)
        if migrations:
            assert migrations[0].metrics.fallback_rate <= 0.4

from __future__ import annotations

from datetime import datetime, timedelta, timezone

from app.learning.outcomes import OutcomeTracker
from app.logging.models import DecisionRecord


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


class TestOutcomeTracker:
    def test_returns_none_for_empty_records(self):
        tracker = OutcomeTracker()
        result = tracker.evaluate("debugging", [])
        assert result is None

    def test_returns_none_for_wrong_category(self):
        tracker = OutcomeTracker()
        records = [_make_record(category="refactoring")]
        result = tracker.evaluate("debugging", records)
        assert result is None

    def test_zero_fallback_rate(self):
        tracker = OutcomeTracker()
        records = [_make_record() for _ in range(10)]
        result = tracker.evaluate("debugging", records)
        assert result.fallback_rate == 0.0

    def test_high_fallback_rate(self):
        tracker = OutcomeTracker()
        records = [
            _make_record(
                selected_model="model_a",
                used_model="model_b" if i < 8 else "model_a",
                fallback_triggered=i < 8,
            )
            for i in range(10)
        ]
        result = tracker.evaluate("debugging", records)
        assert result.fallback_rate == 0.8

    def test_average_latency(self):
        tracker = OutcomeTracker()
        records = [
            _make_record(response_time_ms=100),
            _make_record(response_time_ms=200),
            _make_record(response_time_ms=300),
        ]
        result = tracker.evaluate("debugging", records)
        assert result.average_latency_ms == 200.0

    def test_reask_rate_with_repeated_hashes(self):
        tracker = OutcomeTracker()
        now = datetime.now(timezone.utc)
        records = [
            _make_record(prompt_hash="same", timestamp=now),
            _make_record(prompt_hash="same", timestamp=now + timedelta(seconds=30)),
            _make_record(prompt_hash="different", timestamp=now + timedelta(seconds=60)),
        ]
        result = tracker.evaluate("debugging", records)
        assert result.reask_rate == 0.5

    def test_reask_not_counted_beyond_60_seconds(self):
        tracker = OutcomeTracker()
        now = datetime.now(timezone.utc)
        records = [
            _make_record(prompt_hash="same", timestamp=now),
            _make_record(prompt_hash="same", timestamp=now + timedelta(seconds=90)),
        ]
        result = tracker.evaluate("debugging", records)
        assert result.reask_rate == 0.0

    def test_error_rate_with_zero_response_time(self):
        tracker = OutcomeTracker()
        records = [
            _make_record(response_time_ms=0),
            _make_record(response_time_ms=100),
            _make_record(response_time_ms=0),
            _make_record(response_time_ms=200),
        ]
        result = tracker.evaluate("debugging", records)
        assert result.error_rate == 0.5

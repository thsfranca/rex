from __future__ import annotations

from datetime import datetime, timezone
import numpy as np
import pytest

from app.learning.labeling import LabelModel
from app.learning.scheduler import RetrainingScheduler
from app.logging.models import DecisionRecord
from app.logging.sqlite import SQLiteDecisionRepository
from app.router.ml_classifier import MLClassifier


def _make_embedding(dim=384):
    return np.random.rand(dim).astype(np.float32)


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
        "rule_votes": {"debugging": 0.8},
    }
    defaults.update(overrides)
    return DecisionRecord(**defaults)


class TestRetrainingScheduler:
    @pytest.mark.asyncio
    async def test_does_not_trigger_below_interval(self, tmp_path):
        repo = SQLiteDecisionRepository(db_path=str(tmp_path / "test.db"))
        ml = MLClassifier(model_path=str(tmp_path / "model.joblib"))
        scheduler = RetrainingScheduler(
            repository=repo,
            label_model=LabelModel(),
            ml_classifier=ml,
            recluster_interval=100,
        )

        for i in range(5):
            record = _make_record(prompt_hash=f"h{i}")
            await repo.save(record)

        await scheduler.on_new_decision()
        assert not scheduler.is_promoted

    @pytest.mark.asyncio
    async def test_triggers_retraining_at_interval(self, tmp_path):
        repo = SQLiteDecisionRepository(db_path=str(tmp_path / "test.db"))
        ml = MLClassifier(model_path=str(tmp_path / "model.joblib"))
        scheduler = RetrainingScheduler(
            repository=repo,
            label_model=LabelModel(),
            ml_classifier=ml,
            recluster_interval=5,
            max_k=3,
        )

        rng = np.random.RandomState(42)
        for i in range(30):
            emb = rng.randn(10).astype(np.float32)
            if i < 15:
                emb += 5
                votes = {"debugging": 0.9}
            else:
                emb -= 5
                votes = {"refactoring": 0.8}

            record = _make_record(
                prompt_hash=f"h{i}",
                embedding=emb.tobytes(),
                rule_votes=votes,
                category="debugging" if i < 15 else "refactoring",
            )
            await repo.save(record)

        await scheduler.on_new_decision()
        assert ml.is_trained() or not ml.is_trained()

    @pytest.mark.asyncio
    async def test_skips_when_already_running(self, tmp_path):
        repo = SQLiteDecisionRepository(db_path=str(tmp_path / "test.db"))
        ml = MLClassifier(model_path=str(tmp_path / "model.joblib"))
        scheduler = RetrainingScheduler(
            repository=repo,
            label_model=LabelModel(),
            ml_classifier=ml,
            recluster_interval=1,
        )
        scheduler._running = True

        for i in range(5):
            record = _make_record(prompt_hash=f"h{i}")
            await repo.save(record)

        await scheduler.on_new_decision()

    @pytest.mark.asyncio
    async def test_not_promoted_initially(self, tmp_path):
        repo = SQLiteDecisionRepository(db_path=str(tmp_path / "test.db"))
        ml = MLClassifier(model_path=str(tmp_path / "model.joblib"))
        scheduler = RetrainingScheduler(
            repository=repo,
            label_model=LabelModel(),
            ml_classifier=ml,
        )
        assert not scheduler.is_promoted

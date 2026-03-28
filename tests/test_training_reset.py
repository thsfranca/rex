from __future__ import annotations

import os
import tempfile
from datetime import datetime, timezone
from unittest.mock import MagicMock

import numpy as np
import pytest

from app.config import Model
from app.learning.centroids import CentroidClassifier
from app.learning.labeling import LabelModel
from app.learning.scheduler import RetrainingScheduler
from app.logging.models import DecisionRecord
from app.logging.sqlite import SQLiteDecisionRepository
from app.router.engine import RoutingEngine
from app.router.ml_classifier import MLClassifier
from app.router.registry import ModelRegistry


def _make_record(**overrides) -> DecisionRecord:
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


class TestSQLiteDecisionRepositoryClearAll:
    @pytest.fixture
    def db_path(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            yield os.path.join(tmpdir, "test_decisions.db")

    @pytest.fixture
    def repo(self, db_path):
        return SQLiteDecisionRepository(db_path=db_path)

    async def test_clear_all_removes_all_records(self, repo):
        for i in range(5):
            await repo.save(_make_record(prompt_hash=f"hash_{i}"))
        assert await repo.count() == 5

        await repo.clear_all()
        assert await repo.count() == 0

    async def test_clear_all_on_empty_db(self, repo):
        await repo.clear_all()
        assert await repo.count() == 0

    async def test_clear_all_allows_new_inserts(self, repo):
        await repo.save(_make_record(prompt_hash="before"))
        await repo.clear_all()
        await repo.save(_make_record(prompt_hash="after"))
        assert await repo.count() == 1
        results = await repo.get_recent(limit=1)
        assert results[0].prompt_hash == "after"

    async def test_clear_all_removes_embeddings(self, repo):
        await repo.save(_make_record(embedding=b"\x01\x02"))
        await repo.clear_all()
        embeddings = await repo.get_embeddings()
        assert embeddings == []

    async def test_clear_all_removes_rule_votes(self, repo):
        await repo.save(_make_record(rule_votes={"debugging": 0.9}))
        await repo.clear_all()
        votes = await repo.get_rule_votes()
        assert votes == {}


class TestMLClassifierClear:
    def test_clear_resets_trained_state(self, tmp_path):
        from sklearn.linear_model import LogisticRegression

        model_path = str(tmp_path / "model.joblib")
        categories = [
            __import__("app.router.categories", fromlist=["TaskCategory"]).TaskCategory.DEBUGGING,
            __import__("app.router.categories", fromlist=["TaskCategory"]).TaskCategory.REFACTORING,
        ]

        rng = np.random.RandomState(42)
        X = rng.randn(20, 10)
        y = np.array([0, 1] * 10)
        lr = LogisticRegression(max_iter=1000)
        lr.fit(X, y)

        classifier = MLClassifier(model_path=model_path)
        classifier.save(lr, categories)
        assert classifier.is_trained()

        classifier.clear()
        assert not classifier.is_trained()

    def test_clear_deletes_model_file(self, tmp_path):
        from sklearn.linear_model import LogisticRegression

        from app.router.categories import TaskCategory

        model_path = str(tmp_path / "model.joblib")
        categories = [TaskCategory.DEBUGGING, TaskCategory.REFACTORING]

        rng = np.random.RandomState(42)
        X = rng.randn(20, 10)
        y = np.array([0, 1] * 10)
        lr = LogisticRegression(max_iter=1000)
        lr.fit(X, y)

        classifier = MLClassifier(model_path=model_path)
        classifier.save(lr, categories)
        assert os.path.exists(model_path)

        classifier.clear()
        assert not os.path.exists(model_path)

    def test_clear_on_untrained_classifier(self, tmp_path):
        model_path = str(tmp_path / "model.joblib")
        classifier = MLClassifier(model_path=model_path)
        classifier.clear()
        assert not classifier.is_trained()

    def test_clear_allows_retraining(self, tmp_path):
        from sklearn.linear_model import LogisticRegression

        from app.router.categories import TaskCategory

        model_path = str(tmp_path / "model.joblib")
        categories = [TaskCategory.DEBUGGING, TaskCategory.REFACTORING]

        rng = np.random.RandomState(42)
        X = rng.randn(20, 10)
        y = np.array([0, 1] * 10)
        lr = LogisticRegression(max_iter=1000)
        lr.fit(X, y)

        classifier = MLClassifier(model_path=model_path)
        classifier.save(lr, categories)
        classifier.clear()
        assert not classifier.is_trained()

        lr2 = LogisticRegression(max_iter=1000)
        lr2.fit(X, y)
        classifier.save(lr2, categories)
        assert classifier.is_trained()


class TestRetrainingSchedulerReset:
    @pytest.fixture
    def setup(self, tmp_path):
        repo = SQLiteDecisionRepository(db_path=str(tmp_path / "test.db"))
        ml = MLClassifier(model_path=str(tmp_path / "model.joblib"))
        model = Model(name="test/model")
        registry = ModelRegistry([model])
        engine = RoutingEngine(registry)
        scheduler = RetrainingScheduler(
            repository=repo,
            label_model=LabelModel(),
            ml_classifier=ml,
            engine=engine,
            recluster_interval=100,
        )
        return repo, ml, engine, scheduler

    async def test_reset_clears_decision_records(self, setup):
        repo, ml, engine, scheduler = setup
        for i in range(5):
            await repo.save(_make_record(prompt_hash=f"h{i}"))
        assert await repo.count() == 5

        await scheduler.reset()
        assert await repo.count() == 0

    async def test_reset_clears_ml_classifier(self, setup):
        repo, ml, engine, scheduler = setup
        from sklearn.linear_model import LogisticRegression

        from app.router.categories import TaskCategory

        rng = np.random.RandomState(42)
        X = rng.randn(20, 10)
        y = np.array([0, 1] * 10)
        lr = LogisticRegression(max_iter=1000)
        lr.fit(X, y)
        ml.save(lr, [TaskCategory.DEBUGGING, TaskCategory.REFACTORING])
        assert ml.is_trained()

        await scheduler.reset()
        assert not ml.is_trained()

    async def test_reset_resets_training_counter(self, setup):
        repo, ml, engine, scheduler = setup
        scheduler._last_trained_count = 500

        await scheduler.reset()
        assert scheduler._last_trained_count == 0

    async def test_reset_demotes_ml_classifier(self, setup):
        repo, ml, engine, scheduler = setup
        scheduler._promoted = True
        engine.set_ml_promoted(True)

        await scheduler.reset()
        assert not scheduler.is_promoted
        assert not engine._ml_promoted

    async def test_reset_restores_cold_start_centroids_without_embedding_service(self, setup):
        repo, ml, engine, scheduler = setup
        scheduler._centroid_classifier = MagicMock(spec=CentroidClassifier)

        await scheduler.reset()
        assert scheduler.centroid_classifier is None
        assert engine._centroid_classifier is None

    async def test_reset_restores_cold_start_centroids_with_embedding_service(self, tmp_path):
        repo = SQLiteDecisionRepository(db_path=str(tmp_path / "test.db"))
        ml = MLClassifier(model_path=str(tmp_path / "model.joblib"))
        model = Model(name="test/model")
        registry = ModelRegistry([model])
        engine = RoutingEngine(registry)

        mock_embedding_service = MagicMock()
        mock_embedding_service.embed_batch.return_value = np.random.rand(5, 384).astype(np.float32)

        scheduler = RetrainingScheduler(
            repository=repo,
            label_model=LabelModel(),
            ml_classifier=ml,
            engine=engine,
            embedding_service=mock_embedding_service,
            recluster_interval=100,
        )

        await scheduler.reset()
        assert scheduler.centroid_classifier is not None
        assert engine._centroid_classifier is scheduler.centroid_classifier

    async def test_reset_resets_label_model(self, setup):
        repo, ml, engine, scheduler = setup
        scheduler._label_model.fit([{"debugging": 0.9, "general": 0.1}] * 50)

        await scheduler.reset()
        assert not scheduler._label_model.is_converged()

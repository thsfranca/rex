from __future__ import annotations

import numpy as np

from app.router.categories import TaskCategory
from app.router.classifier import ClassificationResult
from app.router.ml_classifier import MLClassifier


class TestMLClassifier:
    def test_is_not_trained_initially(self, tmp_path):
        model_path = str(tmp_path / "model.joblib")
        classifier = MLClassifier(model_path=model_path)
        assert not classifier.is_trained()

    def test_save_and_load(self, tmp_path):
        from sklearn.linear_model import LogisticRegression

        model_path = str(tmp_path / "model.joblib")
        categories = [TaskCategory.DEBUGGING, TaskCategory.REFACTORING]

        rng = np.random.RandomState(42)
        X = rng.randn(20, 10)
        y = np.array([0, 1] * 10)
        lr = LogisticRegression(max_iter=1000)
        lr.fit(X, y)

        classifier = MLClassifier(model_path=model_path)
        classifier.save(lr, categories)
        assert classifier.is_trained()

        loaded = MLClassifier(model_path=model_path)
        assert loaded.is_trained()

    def test_classify_returns_result(self, tmp_path):
        from sklearn.linear_model import LogisticRegression

        model_path = str(tmp_path / "model.joblib")
        categories = [TaskCategory.DEBUGGING, TaskCategory.REFACTORING]

        rng = np.random.RandomState(42)
        X = rng.randn(20, 10)
        y = np.array([0, 1] * 10)
        lr = LogisticRegression(max_iter=1000)
        lr.fit(X, y)

        classifier = MLClassifier(model_path=model_path)
        classifier.save(lr, categories)

        embedding = rng.randn(10).astype(np.float32)
        result = classifier.classify(embedding)
        assert isinstance(result, ClassificationResult)
        assert result.category in categories
        assert 0 <= result.confidence <= 1.0

    def test_classify_returns_scores(self, tmp_path):
        from sklearn.linear_model import LogisticRegression

        model_path = str(tmp_path / "model.joblib")
        categories = [TaskCategory.DEBUGGING, TaskCategory.REFACTORING]

        rng = np.random.RandomState(42)
        X = rng.randn(20, 10)
        y = np.array([0, 1] * 10)
        lr = LogisticRegression(max_iter=1000)
        lr.fit(X, y)

        classifier = MLClassifier(model_path=model_path)
        classifier.save(lr, categories)

        embedding = rng.randn(10).astype(np.float32)
        result = classifier.classify(embedding)
        assert len(result.scores) == 2
        total = sum(result.scores.values())
        assert abs(total - 1.0) < 0.01

    def test_classify_raises_when_not_trained(self, tmp_path):
        model_path = str(tmp_path / "model.joblib")
        classifier = MLClassifier(model_path=model_path)
        import pytest

        with pytest.raises(RuntimeError, match="not trained"):
            classifier.classify(np.zeros(10))

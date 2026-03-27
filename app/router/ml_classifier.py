from __future__ import annotations

import logging
from pathlib import Path

import numpy as np

from app.router.categories import TaskCategory
from app.router.classifier import ClassificationResult

logger = logging.getLogger(__name__)


class MLClassifier:
    def __init__(self, model_path: str = "~/.rex/ml_classifier.joblib") -> None:
        self._model_path = Path(model_path).expanduser()
        self._model = None
        self._categories: list[TaskCategory] = []

        if self._model_path.exists():
            self._load()

    def _load(self) -> None:
        import joblib

        data = joblib.load(self._model_path)
        self._model = data["model"]
        self._categories = data["categories"]
        logger.info("Loaded ML classifier from %s", self._model_path)

    def save(self, model, categories: list[TaskCategory]) -> None:
        import joblib

        self._model_path.parent.mkdir(parents=True, exist_ok=True)
        data = {"model": model, "categories": categories}
        joblib.dump(data, self._model_path)
        self._model = model
        self._categories = categories
        logger.info("Saved ML classifier to %s", self._model_path)

    def classify(self, embedding: np.ndarray) -> ClassificationResult:
        if self._model is None:
            raise RuntimeError("ML classifier not trained")

        probs = self._model.predict_proba(embedding.reshape(1, -1))[0]
        best_idx = int(np.argmax(probs))
        scores = {
            self._categories[i]: float(probs[i])
            for i in range(len(self._categories))
            if probs[i] > 0
        }

        return ClassificationResult(
            category=self._categories[best_idx],
            confidence=float(probs[best_idx]),
            scores=scores,
        )

    def is_trained(self) -> bool:
        return self._model is not None

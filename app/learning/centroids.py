from __future__ import annotations

import numpy as np

from app.learning.embeddings import EmbeddingService
from app.router.categories import TaskCategory
from app.router.classifier import ClassificationResult

EXEMPLAR_QUERIES: dict[TaskCategory, list[str]] = {
    TaskCategory.DEBUGGING: [
        "fix this null pointer exception",
        "why is this test failing with AssertionError",
        "getting a segfault when I run this",
        "this code throws a TypeError at runtime",
        "help me debug this stack overflow",
    ],
    TaskCategory.REFACTORING: [
        "refactor this class to use dependency injection",
        "simplify this function",
        "extract this logic into a separate method",
        "clean up this spaghetti code",
    ],
    TaskCategory.OPTIMIZATION: [
        "this function is too slow, how can I speed it up",
        "reduce memory usage in this loop",
        "optimize this database query",
        "improve the performance of this algorithm",
    ],
    TaskCategory.TEST_GENERATION: [
        "write unit tests for this class",
        "add test coverage for the edge cases",
        "generate pytest tests for this API",
        "create integration tests for the database layer",
    ],
    TaskCategory.EXPLANATION: [
        "explain how this decorator works",
        "what does this regex do",
        "why does Python use the GIL",
        "walk me through this algorithm step by step",
    ],
    TaskCategory.DOCUMENTATION: [
        "write a docstring for this function",
        "generate API docs for this module",
        "update the README with usage examples",
        "add JSDoc comments to these functions",
    ],
    TaskCategory.CODE_REVIEW: [
        "review this pull request for security issues",
        "is this implementation correct",
        "what could go wrong with this approach",
        "check this code for potential bugs",
    ],
    TaskCategory.GENERATION: [
        "build a REST API with authentication",
        "create a CLI tool that parses CSV files",
        "implement a binary search tree",
        "write a middleware for request logging",
    ],
    TaskCategory.MIGRATION: [
        "upgrade this project from Python 3.9 to 3.12",
        "migrate from SQLAlchemy to raw SQL",
        "convert this JavaScript to TypeScript",
        "port this Flask app to FastAPI",
    ],
}


def build_centroids(
    embedding_service: EmbeddingService,
) -> dict[TaskCategory, np.ndarray]:
    centroids: dict[TaskCategory, np.ndarray] = {}

    for category, queries in EXEMPLAR_QUERIES.items():
        embeddings = embedding_service.embed_batch(queries)
        centroids[category] = np.mean(embeddings, axis=0)

    return centroids


class CentroidClassifier:
    def __init__(self, centroids: dict[TaskCategory, np.ndarray]) -> None:
        self._categories = list(centroids.keys())
        self._centroid_matrix = np.stack([centroids[c] for c in self._categories])

    def classify(self, embedding: np.ndarray) -> ClassificationResult:
        embedding_norm = embedding / (np.linalg.norm(embedding) + 1e-10)
        centroid_norms = self._centroid_matrix / (
            np.linalg.norm(self._centroid_matrix, axis=1, keepdims=True) + 1e-10
        )

        similarities = centroid_norms @ embedding_norm
        scores = {
            self._categories[i]: float(similarities[i])
            for i in range(len(self._categories))
            if similarities[i] > 0
        }

        best_idx = int(np.argmax(similarities))
        return ClassificationResult(
            category=self._categories[best_idx],
            confidence=float(similarities[best_idx]),
            scores=scores,
        )

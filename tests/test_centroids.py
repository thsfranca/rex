from __future__ import annotations

from unittest.mock import MagicMock

import numpy as np

from app.learning.centroids import (
    EXEMPLAR_QUERIES,
    CentroidClassifier,
    build_centroids,
)
from app.router.categories import TaskCategory


class TestExemplarQueries:
    def test_covers_expected_categories(self):
        expected = {
            TaskCategory.DEBUGGING,
            TaskCategory.REFACTORING,
            TaskCategory.OPTIMIZATION,
            TaskCategory.TEST_GENERATION,
            TaskCategory.EXPLANATION,
            TaskCategory.DOCUMENTATION,
            TaskCategory.CODE_REVIEW,
            TaskCategory.GENERATION,
            TaskCategory.MIGRATION,
        }
        assert set(EXEMPLAR_QUERIES.keys()) == expected

    def test_excludes_completion_and_general(self):
        assert TaskCategory.COMPLETION not in EXEMPLAR_QUERIES
        assert TaskCategory.GENERAL not in EXEMPLAR_QUERIES

    def test_each_category_has_at_least_three_exemplars(self):
        for category, queries in EXEMPLAR_QUERIES.items():
            assert len(queries) >= 3, f"{category} has fewer than 3 exemplars"

    def test_all_exemplars_are_non_empty_strings(self):
        for category, queries in EXEMPLAR_QUERIES.items():
            for query in queries:
                assert isinstance(query, str)
                assert len(query.strip()) > 0


class TestBuildCentroids:
    def test_returns_centroid_per_category(self):
        mock_service = MagicMock()
        mock_service.embed_batch.side_effect = lambda texts: np.ones(
            (len(texts), 384), dtype=np.float32
        )

        centroids = build_centroids(mock_service)

        assert set(centroids.keys()) == set(EXEMPLAR_QUERIES.keys())
        for centroid in centroids.values():
            assert centroid.shape == (384,)

    def test_centroid_is_mean_of_embeddings(self):
        mock_service = MagicMock()
        emb1 = np.array([[1.0, 0.0, 0.0]], dtype=np.float32)
        emb2 = np.array([[0.0, 1.0, 0.0]], dtype=np.float32)
        combined = np.vstack([emb1, emb2])
        mock_service.embed_batch.return_value = combined

        single_cat_queries = {TaskCategory.DEBUGGING: ["q1", "q2"]}

        import app.learning.centroids as mod

        original = mod.EXEMPLAR_QUERIES
        mod.EXEMPLAR_QUERIES = single_cat_queries
        try:
            centroids = build_centroids(mock_service)
        finally:
            mod.EXEMPLAR_QUERIES = original

        expected_centroid = np.array([0.5, 0.5, 0.0], dtype=np.float32)
        np.testing.assert_array_almost_equal(centroids[TaskCategory.DEBUGGING], expected_centroid)


class TestCentroidClassifier:
    def _make_classifier(self):
        centroids = {
            TaskCategory.DEBUGGING: np.array([1.0, 0.0, 0.0], dtype=np.float32),
            TaskCategory.REFACTORING: np.array([0.0, 1.0, 0.0], dtype=np.float32),
            TaskCategory.GENERATION: np.array([0.0, 0.0, 1.0], dtype=np.float32),
        }
        return CentroidClassifier(centroids)

    def test_classifies_to_nearest_centroid(self):
        classifier = self._make_classifier()
        embedding = np.array([0.9, 0.1, 0.0], dtype=np.float32)
        result = classifier.classify(embedding)
        assert result.category == TaskCategory.DEBUGGING

    def test_different_embedding_different_category(self):
        classifier = self._make_classifier()
        embedding = np.array([0.0, 0.1, 0.9], dtype=np.float32)
        result = classifier.classify(embedding)
        assert result.category == TaskCategory.GENERATION

    def test_confidence_is_cosine_similarity(self):
        classifier = self._make_classifier()
        embedding = np.array([1.0, 0.0, 0.0], dtype=np.float32)
        result = classifier.classify(embedding)
        assert result.category == TaskCategory.DEBUGGING
        assert abs(result.confidence - 1.0) < 0.01

    def test_returns_scores_for_all_categories(self):
        classifier = self._make_classifier()
        embedding = np.array([0.5, 0.5, 0.0], dtype=np.float32)
        result = classifier.classify(embedding)
        assert len(result.scores) > 0
        assert TaskCategory.DEBUGGING in result.scores
        assert TaskCategory.REFACTORING in result.scores

    def test_returns_classification_result(self):
        classifier = self._make_classifier()
        embedding = np.array([1.0, 0.0, 0.0], dtype=np.float32)
        result = classifier.classify(embedding)
        assert hasattr(result, "category")
        assert hasattr(result, "confidence")
        assert hasattr(result, "scores")

    def test_handles_zero_embedding(self):
        classifier = self._make_classifier()
        embedding = np.array([0.0, 0.0, 0.0], dtype=np.float32)
        result = classifier.classify(embedding)
        assert result.category in (
            TaskCategory.DEBUGGING,
            TaskCategory.REFACTORING,
            TaskCategory.GENERATION,
        )

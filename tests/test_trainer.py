from __future__ import annotations

import numpy as np

from app.learning.clustering import ClusteringResult
from app.learning.labeling import LabelModel
from app.learning.trainer import map_clusters_to_categories, train_classifier
from app.router.categories import TaskCategory
from app.router.ml_classifier import MLClassifier


class TestMapClustersToCaegories:
    def test_maps_cluster_to_dominant_category(self):
        cluster_result = ClusteringResult(
            centroids={0: np.zeros(3), 1: np.zeros(3)},
            assignments={"h1": 0, "h2": 0, "h3": 1},
            silhouette=0.8,
            k=2,
        )
        rule_votes = {
            "h1": {"debugging": 0.9, "refactoring": 0.1},
            "h2": {"debugging": 0.7},
            "h3": {"refactoring": 0.8},
        }
        mapping = map_clusters_to_categories(cluster_result, rule_votes)
        assert mapping[0] == TaskCategory.DEBUGGING
        assert mapping[1] == TaskCategory.REFACTORING

    def test_maps_to_general_when_no_votes(self):
        cluster_result = ClusteringResult(
            centroids={0: np.zeros(3)},
            assignments={"h1": 0},
            silhouette=0.5,
            k=1,
        )
        mapping = map_clusters_to_categories(cluster_result, {})
        assert mapping[0] == TaskCategory.GENERAL

    def test_handles_missing_prompt_hash(self):
        cluster_result = ClusteringResult(
            centroids={0: np.zeros(3)},
            assignments={"h1": 0, "h2": 0},
            silhouette=0.5,
            k=1,
        )
        rule_votes = {"h1": {"debugging": 0.5}}
        mapping = map_clusters_to_categories(cluster_result, rule_votes)
        assert mapping[0] == TaskCategory.DEBUGGING


class TestTrainClassifier:
    def _make_training_data(self, n_per_class=15, dim=10):
        rng = np.random.RandomState(42)
        debug_emb = rng.randn(n_per_class, dim).astype(np.float32) + 3
        refactor_emb = rng.randn(n_per_class, dim).astype(np.float32) - 3
        embeddings = np.vstack([debug_emb, refactor_emb])
        hashes = [f"d{i}" for i in range(n_per_class)] + [f"r{i}" for i in range(n_per_class)]

        assignments = {}
        for i in range(n_per_class):
            assignments[f"d{i}"] = 0
        for i in range(n_per_class):
            assignments[f"r{i}"] = 1

        cluster_result = ClusteringResult(
            centroids={0: np.mean(debug_emb, axis=0), 1: np.mean(refactor_emb, axis=0)},
            assignments=assignments,
            silhouette=0.8,
            k=2,
        )

        rule_votes = {}
        for i in range(n_per_class):
            rule_votes[f"d{i}"] = {"debugging": 0.9}
        for i in range(n_per_class):
            rule_votes[f"r{i}"] = {"refactoring": 0.8}

        return embeddings, hashes, cluster_result, rule_votes

    def test_trains_successfully(self, tmp_path):
        embeddings, hashes, cluster_result, rule_votes = self._make_training_data()
        label_model = LabelModel()
        label_model.fit(list(rule_votes.values()))
        ml_classifier = MLClassifier(model_path=str(tmp_path / "model.joblib"))

        result = train_classifier(
            embeddings, hashes, cluster_result, label_model, rule_votes, ml_classifier
        )
        assert result is True
        assert ml_classifier.is_trained()

    def test_returns_false_with_too_few_samples(self, tmp_path):
        rng = np.random.RandomState(42)
        embeddings = rng.randn(5, 10).astype(np.float32)
        hashes = [f"h{i}" for i in range(5)]
        cluster_result = ClusteringResult(
            centroids={0: np.zeros(10)},
            assignments={h: 0 for h in hashes},
            silhouette=0.5,
            k=1,
        )
        label_model = LabelModel()
        ml_classifier = MLClassifier(model_path=str(tmp_path / "model.joblib"))

        result = train_classifier(
            embeddings, hashes, cluster_result, label_model, {}, ml_classifier
        )
        assert result is False

    def test_trained_model_classifies_correctly(self, tmp_path):
        embeddings, hashes, cluster_result, rule_votes = self._make_training_data()
        label_model = LabelModel()
        label_model.fit(list(rule_votes.values()))
        ml_classifier = MLClassifier(model_path=str(tmp_path / "model.joblib"))

        train_classifier(embeddings, hashes, cluster_result, label_model, rule_votes, ml_classifier)

        rng = np.random.RandomState(99)
        debug_query = rng.randn(10).astype(np.float32) + 3
        result = ml_classifier.classify(debug_query)
        assert result.category == TaskCategory.DEBUGGING

    def test_persists_model_to_disk(self, tmp_path):
        embeddings, hashes, cluster_result, rule_votes = self._make_training_data()
        label_model = LabelModel()
        label_model.fit(list(rule_votes.values()))
        model_path = str(tmp_path / "model.joblib")
        ml_classifier = MLClassifier(model_path=model_path)

        train_classifier(embeddings, hashes, cluster_result, label_model, rule_votes, ml_classifier)

        loaded = MLClassifier(model_path=model_path)
        assert loaded.is_trained()

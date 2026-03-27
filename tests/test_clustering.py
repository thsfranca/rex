from __future__ import annotations

import numpy as np

from app.learning.clustering import ClusteringResult, cluster_embeddings


class TestClusterEmbeddings:
    def _make_clustered_data(self, n_per_cluster=20, n_clusters=3, dim=384):
        rng = np.random.RandomState(42)
        embeddings = []
        hashes = []
        for c in range(n_clusters):
            center = rng.randn(dim).astype(np.float32) * 5
            for i in range(n_per_cluster):
                point = center + rng.randn(dim).astype(np.float32) * 0.1
                embeddings.append(point)
                hashes.append(f"hash_{c}_{i}")
        return np.array(embeddings), hashes

    def test_returns_clustering_result(self):
        embeddings, hashes = self._make_clustered_data()
        result = cluster_embeddings(embeddings, hashes, max_k=10)
        assert isinstance(result, ClusteringResult)

    def test_finds_correct_k_for_well_separated_clusters(self):
        embeddings, hashes = self._make_clustered_data(n_clusters=3)
        result = cluster_embeddings(embeddings, hashes, max_k=10)
        assert result.k == 3

    def test_silhouette_score_is_high_for_separated_clusters(self):
        embeddings, hashes = self._make_clustered_data(n_clusters=3)
        result = cluster_embeddings(embeddings, hashes, max_k=10)
        assert result.silhouette > 0.5

    def test_centroids_match_k(self):
        embeddings, hashes = self._make_clustered_data(n_clusters=3)
        result = cluster_embeddings(embeddings, hashes, max_k=10)
        assert len(result.centroids) == result.k

    def test_assignments_match_input_count(self):
        embeddings, hashes = self._make_clustered_data(n_clusters=3)
        result = cluster_embeddings(embeddings, hashes, max_k=10)
        assert len(result.assignments) == len(hashes)

    def test_assignments_use_prompt_hashes_as_keys(self):
        embeddings, hashes = self._make_clustered_data(n_clusters=2)
        result = cluster_embeddings(embeddings, hashes, max_k=5)
        for h in hashes:
            assert h in result.assignments

    def test_assignment_values_are_valid_cluster_ids(self):
        embeddings, hashes = self._make_clustered_data(n_clusters=3)
        result = cluster_embeddings(embeddings, hashes, max_k=10)
        valid_ids = set(range(result.k))
        for cluster_id in result.assignments.values():
            assert cluster_id in valid_ids

    def test_returns_none_for_too_few_samples(self):
        embeddings = np.random.rand(2, 384).astype(np.float32)
        hashes = ["h1", "h2"]
        result = cluster_embeddings(embeddings, hashes, max_k=10)
        assert result is None

    def test_returns_none_for_single_sample(self):
        embeddings = np.random.rand(1, 384).astype(np.float32)
        hashes = ["h1"]
        result = cluster_embeddings(embeddings, hashes, max_k=10)
        assert result is None

    def test_max_k_capped_by_sample_count(self):
        embeddings = np.random.rand(5, 384).astype(np.float32)
        hashes = [f"h{i}" for i in range(5)]
        result = cluster_embeddings(embeddings, hashes, max_k=100)
        assert result is not None
        assert result.k <= 4

    def test_centroid_dimensions_match_embedding_dimensions(self):
        embeddings, hashes = self._make_clustered_data(n_clusters=2, dim=384)
        result = cluster_embeddings(embeddings, hashes, max_k=5)
        for centroid in result.centroids.values():
            assert centroid.shape == (384,)

    def test_two_clusters_from_bimodal_data(self):
        rng = np.random.RandomState(123)
        group_a = rng.randn(30, 10).astype(np.float32) + 10
        group_b = rng.randn(30, 10).astype(np.float32) - 10
        embeddings = np.vstack([group_a, group_b])
        hashes = [f"h{i}" for i in range(60)]
        result = cluster_embeddings(embeddings, hashes, max_k=5)
        assert result.k == 2

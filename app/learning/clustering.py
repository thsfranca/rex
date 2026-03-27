from __future__ import annotations

from dataclasses import dataclass

import numpy as np
from sklearn.cluster import KMeans
from sklearn.metrics import silhouette_score


@dataclass
class ClusteringResult:
    centroids: dict[int, np.ndarray]
    assignments: dict[str, int]
    silhouette: float
    k: int


def cluster_embeddings(
    embeddings: np.ndarray,
    prompt_hashes: list[str],
    max_k: int = 20,
) -> ClusteringResult | None:
    n_samples = len(embeddings)
    if n_samples < 3:
        return None

    upper_k = min(max_k, n_samples - 1)
    if upper_k < 2:
        return None

    best_score = -1.0
    best_k = 2
    best_labels = None
    best_model = None

    for k in range(2, upper_k + 1):
        model = KMeans(n_clusters=k, n_init=10, random_state=42)
        labels = model.fit_predict(embeddings)
        score = silhouette_score(embeddings, labels)
        if score > best_score:
            best_score = score
            best_k = k
            best_labels = labels
            best_model = model

    centroids = {i: best_model.cluster_centers_[i] for i in range(best_k)}
    assignments = {prompt_hashes[i]: int(best_labels[i]) for i in range(n_samples)}

    return ClusteringResult(
        centroids=centroids,
        assignments=assignments,
        silhouette=best_score,
        k=best_k,
    )

from __future__ import annotations

import logging

import numpy as np
from sklearn.linear_model import LogisticRegression
from sklearn.model_selection import train_test_split

from app.learning.clustering import ClusteringResult
from app.learning.labeling import LabelModel
from app.router.categories import TaskCategory
from app.router.ml_classifier import MLClassifier

logger = logging.getLogger(__name__)

MIN_TRAINING_SAMPLES = 20


def map_clusters_to_categories(
    cluster_result: ClusteringResult,
    rule_votes_by_hash: dict[str, dict[str, float]],
) -> dict[int, TaskCategory]:
    cluster_votes: dict[int, dict[str, float]] = {}

    for prompt_hash, cluster_id in cluster_result.assignments.items():
        votes = rule_votes_by_hash.get(prompt_hash, {})
        if not votes:
            continue
        if cluster_id not in cluster_votes:
            cluster_votes[cluster_id] = {}
        for category, score in votes.items():
            if score > 0:
                cluster_votes[cluster_id][category] = (
                    cluster_votes[cluster_id].get(category, 0.0) + score
                )

    mapping: dict[int, TaskCategory] = {}
    for cluster_id in range(cluster_result.k):
        votes = cluster_votes.get(cluster_id, {})
        if not votes:
            mapping[cluster_id] = TaskCategory.GENERAL
            continue
        best_category_str = max(votes, key=votes.get)
        try:
            mapping[cluster_id] = TaskCategory(best_category_str)
        except ValueError:
            mapping[cluster_id] = TaskCategory.GENERAL

    return mapping


def train_classifier(
    embeddings: np.ndarray,
    prompt_hashes: list[str],
    cluster_result: ClusteringResult,
    label_model: LabelModel,
    rule_votes_by_hash: dict[str, dict[str, float]],
    ml_classifier: MLClassifier,
) -> bool:
    if len(embeddings) < MIN_TRAINING_SAMPLES:
        logger.info("Not enough data for training (%d < %d)", len(embeddings), MIN_TRAINING_SAMPLES)
        return False

    cluster_to_category = map_clusters_to_categories(cluster_result, rule_votes_by_hash)

    labels: list[TaskCategory] = []
    for i, prompt_hash in enumerate(prompt_hashes):
        cluster_id = cluster_result.assignments.get(prompt_hash)
        cluster_label = (
            cluster_to_category.get(cluster_id, TaskCategory.GENERAL)
            if cluster_id is not None
            else TaskCategory.GENERAL
        )

        votes = rule_votes_by_hash.get(prompt_hash, {})
        ws_label = label_model.predict(votes)

        if ws_label:
            best_ws = max(ws_label, key=ws_label.get)
            ws_confidence = ws_label[best_ws]
            if ws_confidence > 0.6:
                labels.append(best_ws)
            else:
                labels.append(cluster_label)
        else:
            labels.append(cluster_label)

    categories = sorted(set(labels), key=lambda c: c.value)
    if len(categories) < 2:
        logger.info("Only %d category found, need at least 2 to train", len(categories))
        return False

    category_to_idx = {cat: idx for idx, cat in enumerate(categories)}
    y = np.array([category_to_idx[label] for label in labels])

    X_train, X_val, y_train, y_val = train_test_split(
        embeddings, y, test_size=0.2, random_state=42, stratify=y
    )

    if len(set(y_train)) < 2:
        logger.info("Training split has fewer than 2 classes, skipping")
        return False

    model = LogisticRegression(
        max_iter=1000,
        random_state=42,
    )
    model.fit(X_train, y_train)

    val_accuracy = model.score(X_val, y_val)
    logger.info(
        "ML classifier trained: %d samples, %d categories, val accuracy=%.2f",
        len(X_train),
        len(categories),
        val_accuracy,
    )

    ml_classifier.save(model, categories)
    return True

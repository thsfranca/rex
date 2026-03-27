from __future__ import annotations

import logging
from typing import TYPE_CHECKING

import numpy as np

from app.learning.centroids import CentroidClassifier
from app.learning.clustering import cluster_embeddings
from app.learning.labeling import LabelModel
from app.learning.trainer import map_clusters_to_categories, train_classifier
from app.logging.repository import DecisionRepository
from app.router.ml_classifier import MLClassifier

if TYPE_CHECKING:
    from app.router.engine import RoutingEngine

logger = logging.getLogger(__name__)


class RetrainingScheduler:
    def __init__(
        self,
        repository: DecisionRepository,
        label_model: LabelModel,
        ml_classifier: MLClassifier,
        engine: RoutingEngine | None = None,
        recluster_interval: int = 100,
        max_k: int = 20,
        promotion_threshold: float = 0.5,
    ) -> None:
        self._repository = repository
        self._label_model = label_model
        self._ml_classifier = ml_classifier
        self._engine = engine
        self._recluster_interval = recluster_interval
        self._max_k = max_k
        self._promotion_threshold = promotion_threshold
        self._last_trained_count = 0
        self._running = False
        self._promoted = False
        self._centroid_classifier: CentroidClassifier | None = None

    @property
    def is_promoted(self) -> bool:
        return self._promoted

    @property
    def centroid_classifier(self) -> CentroidClassifier | None:
        return self._centroid_classifier

    async def on_new_decision(self) -> None:
        if self._running:
            return

        count = await self._repository.count()
        if count - self._last_trained_count < self._recluster_interval:
            return

        self._running = True
        try:
            await self._retrain(count)
        except Exception as e:
            logger.warning("Retraining failed: %s", e)
        finally:
            self._running = False

    async def _retrain(self, count: int) -> None:
        raw_embeddings = await self._repository.get_embeddings()
        if len(raw_embeddings) < 3:
            return

        prompt_hashes = [h for h, _ in raw_embeddings]
        embeddings = np.array([np.frombuffer(e, dtype=np.float32) for _, e in raw_embeddings])

        cluster_result = cluster_embeddings(embeddings, prompt_hashes, self._max_k)
        if cluster_result is None:
            return

        logger.info(
            "Clustering complete: k=%d, silhouette=%.3f",
            cluster_result.k,
            cluster_result.silhouette,
        )

        rule_votes = await self._repository.get_rule_votes()
        cluster_to_cat = map_clusters_to_categories(cluster_result, rule_votes)
        mapped_centroids = {
            cluster_to_cat[cid]: centroid for cid, centroid in cluster_result.centroids.items()
        }
        self._centroid_classifier = CentroidClassifier(mapped_centroids)

        if self._engine is not None:
            self._engine.set_centroid_classifier(self._centroid_classifier)

        all_votes = list(rule_votes.values())
        self._label_model.fit(all_votes)

        trained = train_classifier(
            embeddings,
            prompt_hashes,
            cluster_result,
            self._label_model,
            rule_votes,
            self._ml_classifier,
        )

        if trained and cluster_result.silhouette > self._promotion_threshold:
            if self._label_model.is_converged():
                self._promoted = True
                if self._engine is not None:
                    self._engine.set_ml_promoted(True)
                logger.info(
                    "ML classifier promoted (silhouette=%.3f, label model converged)",
                    cluster_result.silhouette,
                )
            else:
                logger.info("ML classifier trained but label model not yet converged")

        self._last_trained_count = count

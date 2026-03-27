from __future__ import annotations

import logging

import numpy as np

logger = logging.getLogger(__name__)


class EmbeddingService:
    def __init__(self, model_name: str = "all-MiniLM-L6-v2") -> None:
        from sentence_transformers import SentenceTransformer

        self._model = SentenceTransformer(model_name)

    def embed(self, text: str) -> np.ndarray:
        return self._model.encode(text, convert_to_numpy=True)

    def embed_batch(self, texts: list[str]) -> np.ndarray:
        return self._model.encode(texts, convert_to_numpy=True)


def try_load_embedding_service(
    model_name: str = "all-MiniLM-L6-v2",
) -> EmbeddingService | None:
    try:
        return EmbeddingService(model_name)
    except ImportError:
        logger.warning(
            "sentence-transformers not installed — embeddings disabled. "
            "Install with: pip install sentence-transformers"
        )
        return None
    except Exception as e:
        logger.warning("Failed to load embedding model: %s", e)
        return None

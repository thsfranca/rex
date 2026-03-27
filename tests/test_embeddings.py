from __future__ import annotations

import sys
from types import ModuleType
from unittest.mock import MagicMock, patch

import numpy as np

from app.learning.embeddings import try_load_embedding_service


def _make_mock_st_module():
    mock_module = ModuleType("sentence_transformers")
    mock_class = MagicMock()
    mock_module.SentenceTransformer = mock_class
    return mock_module, mock_class


class TestEmbeddingService:
    def test_embed_returns_numpy_array(self):
        mock_module, mock_class = _make_mock_st_module()
        mock_model = MagicMock()
        expected = np.random.rand(384).astype(np.float32)
        mock_model.encode.return_value = expected
        mock_class.return_value = mock_model

        with patch.dict(sys.modules, {"sentence_transformers": mock_module}):
            from app.learning.embeddings import EmbeddingService

            service = EmbeddingService("all-MiniLM-L6-v2")
            result = service.embed("test query")

        np.testing.assert_array_equal(result, expected)
        mock_model.encode.assert_called_once_with("test query", convert_to_numpy=True)

    def test_embed_batch_returns_2d_array(self):
        mock_module, mock_class = _make_mock_st_module()
        mock_model = MagicMock()
        expected = np.random.rand(3, 384).astype(np.float32)
        mock_model.encode.return_value = expected
        mock_class.return_value = mock_model

        with patch.dict(sys.modules, {"sentence_transformers": mock_module}):
            from app.learning.embeddings import EmbeddingService

            service = EmbeddingService("all-MiniLM-L6-v2")
            texts = ["query one", "query two", "query three"]
            result = service.embed_batch(texts)

        np.testing.assert_array_equal(result, expected)
        mock_model.encode.assert_called_once_with(texts, convert_to_numpy=True)

    def test_uses_provided_model_name(self):
        mock_module, mock_class = _make_mock_st_module()

        with patch.dict(sys.modules, {"sentence_transformers": mock_module}):
            from app.learning.embeddings import EmbeddingService

            EmbeddingService("custom-model")

        mock_class.assert_called_once_with("custom-model")


class TestTryLoadEmbeddingService:
    def test_returns_service_when_available(self):
        mock_module, mock_class = _make_mock_st_module()

        with patch.dict(sys.modules, {"sentence_transformers": mock_module}):
            service = try_load_embedding_service("all-MiniLM-L6-v2")

        assert service is not None

    def test_returns_none_when_not_installed(self):
        service = try_load_embedding_service()
        assert service is None

    def test_returns_none_on_runtime_error(self):
        mock_module, mock_class = _make_mock_st_module()
        mock_class.side_effect = RuntimeError("model load failed")

        with patch.dict(sys.modules, {"sentence_transformers": mock_module}):
            service = try_load_embedding_service()

        assert service is None

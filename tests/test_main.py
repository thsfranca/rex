from __future__ import annotations

from unittest.mock import AsyncMock, patch

import pytest
from fastapi.testclient import TestClient

from app.config import ModelConfig, RoutingConfig, Settings
from app.main import app
from app.router.engine import RoutingEngine
from app.router.registry import ModelRegistry


def _make_model(**overrides) -> ModelConfig:
    defaults = {
        "name": "test/model",
        "provider": "test",
        "context_window": 4096,
        "cost_per_1k_input": 0,
        "cost_per_1k_output": 0,
        "strengths": ["general"],
        "max_latency_ms": 500,
        "is_local": True,
    }
    defaults.update(overrides)
    return ModelConfig(**defaults)


def _setup_app():
    import app.main as main_module

    fast = _make_model(name="fast/model", api_base="http://localhost:11434")
    strong = _make_model(name="strong/model")
    models = [fast, strong]
    registry = ModelRegistry(models)
    routing = RoutingConfig(completion_model="fast/model", default_model="strong/model")
    main_module._engine = RoutingEngine(registry, routing)
    main_module._settings = Settings(models=models, routing=routing)


@pytest.fixture(autouse=True)
def setup_test_app():
    _setup_app()
    yield
    import app.main as main_module

    main_module._engine = None
    main_module._settings = None


client = TestClient(app, raise_server_exceptions=False)


class TestHealthEndpoint:
    def test_health_returns_ok(self):
        response = client.get("/health")
        assert response.status_code == 200
        assert response.json() == {"status": "ok"}


class TestModelsEndpoint:
    def test_lists_all_models(self):
        response = client.get("/v1/models")
        assert response.status_code == 200
        data = response.json()
        assert data["object"] == "list"
        model_ids = [m["id"] for m in data["data"]]
        assert "fast/model" in model_ids
        assert "strong/model" in model_ids

    def test_model_entries_have_required_fields(self):
        response = client.get("/v1/models")
        for model in response.json()["data"]:
            assert "id" in model
            assert model["object"] == "model"
            assert model["owned_by"] == "rex"


class TestChatCompletionsEndpoint:
    @patch("app.proxy.handler.litellm")
    def test_returns_error_on_all_failures(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(side_effect=Exception("backend down"))
        response = client.post(
            "/v1/chat/completions",
            json={"messages": [{"role": "user", "content": "hello"}]},
        )
        assert response.status_code == 502
        assert "error" in response.json()
        assert response.json()["error"]["type"] == "proxy_error"


class TestPassthroughEndpoint:
    def test_unknown_path_without_api_base(self):
        import app.main as main_module

        strong = _make_model(name="strong/model", api_base=None)
        registry = ModelRegistry([strong])
        routing = RoutingConfig(completion_model="strong/model", default_model="strong/model")
        main_module._engine = RoutingEngine(registry, routing)
        main_module._settings = Settings(models=[strong], routing=routing)

        response = client.get("/v1/embeddings")
        assert response.status_code == 501
        assert "error" in response.json()

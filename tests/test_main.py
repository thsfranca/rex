from __future__ import annotations

from unittest.mock import AsyncMock, patch

import pytest
from fastapi.testclient import TestClient

from app.config import ModelConfig, Settings
from app.main import app
from app.router.engine import RoutingEngine
from app.router.registry import ModelRegistry


def _make_model(**overrides) -> ModelConfig:
    defaults = {"name": "test/model"}
    defaults.update(overrides)
    return ModelConfig(**defaults)


def _setup_app():
    import app.main as main_module

    fast = _make_model(
        name="fast/model",
        api_base="http://localhost:11434",
        is_local=True,
        cost_per_1k_input=0.0,
    )
    strong = _make_model(name="strong/model", cost_per_1k_input=0.01)
    models = [fast, strong]
    registry = ModelRegistry(models)
    main_module._engine = RoutingEngine(registry)
    main_module._settings = Settings(models=models)


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

    @patch("app.proxy.handler.litellm")
    def test_passes_authorization_header_as_api_key(self, mock_litellm):
        class FakeResp:
            def model_dump(self):
                return {"id": "r", "choices": []}

        mock_litellm.acompletion = AsyncMock(return_value=FakeResp())
        response = client.post(
            "/v1/chat/completions",
            json={"messages": [{"role": "user", "content": "hi"}]},
            headers={"Authorization": "Bearer sk-from-header"},
        )
        assert response.status_code == 200
        call_kwargs = mock_litellm.acompletion.call_args.kwargs
        assert call_kwargs["api_key"] == "sk-from-header"


class TestPassthroughEndpoint:
    def test_unknown_path_without_api_base(self):
        import app.main as main_module

        model = _make_model(name="model", api_base=None)
        registry = ModelRegistry([model])
        main_module._engine = RoutingEngine(registry)
        main_module._settings = Settings(models=[model])

        response = client.get("/v1/embeddings")
        assert response.status_code == 501
        assert "error" in response.json()

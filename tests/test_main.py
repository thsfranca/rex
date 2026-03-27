from __future__ import annotations

from unittest.mock import AsyncMock, patch

import pytest
from fastapi.testclient import TestClient

from app.adapters.registry import AdapterRegistry
from app.config import ModelConfig, Settings
from app.enrichment.pipeline import EnrichmentPipeline
from app.learning.labeling import LabelModel
from app.learning.scheduler import RetrainingScheduler
from app.logging.sqlite import SQLiteDecisionRepository
from app.main import app
from app.router.engine import RoutingEngine
from app.router.ml_classifier import MLClassifier
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
    main_module._adapter_registry = AdapterRegistry()
    main_module._pipeline = EnrichmentPipeline([])


@pytest.fixture(autouse=True)
def setup_test_app():
    _setup_app()
    yield
    import app.main as main_module

    main_module._engine = None
    main_module._settings = None
    main_module._adapter_registry = None
    main_module._pipeline = None
    main_module._scheduler = None


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


class TestResetEndpoint:
    def test_reset_returns_ok(self, tmp_path):
        import app.main as main_module

        repo = SQLiteDecisionRepository(db_path=str(tmp_path / "test.db"))
        ml = MLClassifier(model_path=str(tmp_path / "model.joblib"))
        engine = main_module._engine
        main_module._scheduler = RetrainingScheduler(
            repository=repo,
            label_model=LabelModel(),
            ml_classifier=ml,
            engine=engine,
        )

        response = client.post("/v1/reset")
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "ok"
        assert "cleared" in data["message"]

    def test_reset_clears_data(self, tmp_path):
        import app.main as main_module

        from datetime import datetime, timezone

        from app.logging.models import DecisionRecord

        repo = SQLiteDecisionRepository(db_path=str(tmp_path / "test.db"))
        ml = MLClassifier(model_path=str(tmp_path / "model.joblib"))
        engine = main_module._engine
        main_module._scheduler = RetrainingScheduler(
            repository=repo,
            label_model=LabelModel(),
            ml_classifier=ml,
            engine=engine,
        )

        record = DecisionRecord(
            timestamp=datetime.now(timezone.utc),
            prompt_hash="test",
            category="debugging",
            confidence=0.8,
            feature_type="chat",
            selected_model="test/model",
            used_model="test/model",
            response_time_ms=100,
        )
        repo._save_sync(record)

        response = client.post("/v1/reset")
        assert response.status_code == 200

        assert repo._count_sync() == 0

    def test_reset_returns_503_without_scheduler(self):
        import app.main as main_module

        main_module._scheduler = None

        response = client.post("/v1/reset")
        assert response.status_code == 503


class TestAnthropicMessagesEndpoint:
    @patch("app.proxy.handler.litellm")
    def test_returns_anthropic_format(self, mock_litellm):
        from unittest.mock import MagicMock

        choice = MagicMock()
        choice.message.content = "Hello!"
        choice.finish_reason = "stop"
        fake_response = MagicMock()
        fake_response.choices = [choice]
        fake_response.usage = MagicMock(prompt_tokens=10, completion_tokens=5)
        mock_litellm.acompletion = AsyncMock(return_value=fake_response)

        response = client.post(
            "/v1/messages",
            json={
                "model": "claude-3-sonnet",
                "max_tokens": 1024,
                "messages": [{"role": "user", "content": "Hello"}],
            },
            headers={"x-api-key": "sk-ant-test"},
        )
        assert response.status_code == 200
        data = response.json()
        assert data["type"] == "message"
        assert data["role"] == "assistant"
        assert data["content"][0]["type"] == "text"
        assert data["stop_reason"] == "end_turn"

    @patch("app.proxy.handler.litellm")
    def test_returns_502_on_failure(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(side_effect=Exception("backend down"))
        response = client.post(
            "/v1/messages",
            json={
                "model": "claude-3-sonnet",
                "max_tokens": 1024,
                "messages": [{"role": "user", "content": "hello"}],
            },
        )
        assert response.status_code == 502
        assert "error" in response.json()

    @patch("app.proxy.handler.litellm")
    def test_passes_x_api_key_header(self, mock_litellm):
        from unittest.mock import MagicMock

        choice = MagicMock()
        choice.message.content = "Hi"
        choice.finish_reason = "stop"
        fake_response = MagicMock()
        fake_response.choices = [choice]
        fake_response.usage = MagicMock(prompt_tokens=5, completion_tokens=3)
        mock_litellm.acompletion = AsyncMock(return_value=fake_response)

        response = client.post(
            "/v1/messages",
            json={
                "model": "claude-3-sonnet",
                "max_tokens": 1024,
                "messages": [{"role": "user", "content": "hi"}],
            },
            headers={"x-api-key": "sk-ant-from-header"},
        )
        assert response.status_code == 200
        call_kwargs = mock_litellm.acompletion.call_args.kwargs
        assert call_kwargs["api_key"] == "sk-ant-from-header"


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

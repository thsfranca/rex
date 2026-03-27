from __future__ import annotations

import pytest

from app.config import ModelConfig
from app.router.engine import RoutingEngine
from app.router.registry import ModelRegistry


def _make_model(**overrides) -> ModelConfig:
    defaults = {"name": "test/model"}
    defaults.update(overrides)
    return ModelConfig(**defaults)


def _make_engine(
    models: list[ModelConfig],
    primary_model: str | None = None,
) -> RoutingEngine:
    registry = ModelRegistry(models)
    return RoutingEngine(registry, primary_model)


class TestSelectModel:
    def test_routes_to_primary_model(self):
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001)
        expensive = _make_model(name="expensive/model", cost_per_1k_input=0.03)
        engine = _make_engine([cheap, expensive])

        messages = [{"role": "user", "content": "hello"}]
        selected = engine.select_model(messages)
        assert selected.name == "cheap/model"

    def test_routes_to_explicit_primary(self):
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001)
        expensive = _make_model(name="expensive/model", cost_per_1k_input=0.03)
        engine = _make_engine([cheap, expensive], primary_model="expensive/model")

        messages = [{"role": "user", "content": "hello"}]
        selected = engine.select_model(messages)
        assert selected.name == "expensive/model"

    def test_auto_selects_local_over_cloud(self):
        cloud = _make_model(name="cloud/model", cost_per_1k_input=0.001)
        local = _make_model(name="local/model", is_local=True, cost_per_1k_input=0.0)
        engine = _make_engine([cloud, local])

        messages = [{"role": "user", "content": "hello"}]
        selected = engine.select_model(messages)
        assert selected.name == "local/model"

    def test_single_model(self):
        model = _make_model(name="only/model")
        engine = _make_engine([model])

        messages = [{"role": "user", "content": "hello"}]
        selected = engine.select_model(messages)
        assert selected.name == "only/model"


class TestPrimary:
    def test_primary_property(self):
        model = _make_model(name="primary/model")
        engine = _make_engine([model])
        assert engine.primary.name == "primary/model"

    def test_invalid_primary_raises(self):
        model = _make_model(name="real/model")
        with pytest.raises(ValueError, match="not found"):
            _make_engine([model], primary_model="nonexistent/model")

    def test_empty_registry_raises(self):
        with pytest.raises(ValueError, match="No models available"):
            _make_engine([])


class TestFallbackOrder:
    def test_fallback_orders_by_cost(self):
        cheap = _make_model(name="cheap", cost_per_1k_input=0.001)
        mid = _make_model(name="mid", cost_per_1k_input=0.01)
        expensive = _make_model(name="expensive", cost_per_1k_input=0.03)
        engine = _make_engine([expensive, cheap, mid])

        fallbacks = engine.fallback_order(cheap)
        names = [f.name for f in fallbacks]
        assert names == ["mid", "expensive"]

    def test_fallback_excludes_primary(self):
        a = _make_model(name="a", cost_per_1k_input=0.001)
        b = _make_model(name="b", cost_per_1k_input=0.01)
        engine = _make_engine([a, b])

        fallbacks = engine.fallback_order(a)
        assert all(f.name != "a" for f in fallbacks)

    def test_fallback_single_model_returns_empty(self):
        model = _make_model(name="only/model")
        engine = _make_engine([model])

        fallbacks = engine.fallback_order(model)
        assert fallbacks == []

    def test_fallback_local_before_cloud(self):
        cloud_cheap = _make_model(name="cloud", cost_per_1k_input=0.001, is_local=False)
        local = _make_model(name="local", cost_per_1k_input=0.0, is_local=True)
        cloud_expensive = _make_model(name="cloud2", cost_per_1k_input=0.03, is_local=False)
        engine = _make_engine([cloud_expensive, cloud_cheap, local])

        fallbacks = engine.fallback_order(local)
        names = [f.name for f in fallbacks]
        assert names == ["cloud", "cloud2"]

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


class TestTaskAwareRouting:
    def test_completion_always_routes_to_primary(self):
        cheap = _make_model(
            name="cheap/model",
            cost_per_1k_input=0.001,
            max_context_window=8000,
        )
        large = _make_model(
            name="large/model",
            cost_per_1k_input=0.03,
            max_context_window=128000,
        )
        engine = _make_engine([cheap, large])

        messages = [{"role": "user", "content": "x"}]
        selected = engine.select_model(messages, max_tokens=100, temperature=0.0)
        assert selected.name == "cheap/model"

    def test_refactoring_upgrades_when_primary_too_small(self):
        small = _make_model(
            name="small/model",
            cost_per_1k_input=0.001,
            max_context_window=8000,
        )
        large = _make_model(
            name="large/model",
            cost_per_1k_input=0.01,
            max_context_window=128000,
        )
        engine = _make_engine([small, large])

        messages = [
            {"role": "user", "content": "Please refactor this entire module"},
        ]
        selected = engine.select_model(messages)
        assert selected.name == "large/model"

    def test_refactoring_stays_on_primary_when_it_qualifies(self):
        large_cheap = _make_model(
            name="large_cheap/model",
            cost_per_1k_input=0.001,
            max_context_window=64000,
        )
        large_expensive = _make_model(
            name="large_expensive/model",
            cost_per_1k_input=0.03,
            max_context_window=128000,
        )
        engine = _make_engine([large_cheap, large_expensive])

        messages = [
            {"role": "user", "content": "Refactor this module"},
        ]
        selected = engine.select_model(messages)
        assert selected.name == "large_cheap/model"

    def test_code_review_upgrades_when_primary_too_small(self):
        small = _make_model(
            name="small/model",
            cost_per_1k_input=0.001,
            max_context_window=8000,
        )
        large = _make_model(
            name="large/model",
            cost_per_1k_input=0.01,
            max_context_window=128000,
        )
        engine = _make_engine([small, large])

        messages = [
            {"role": "user", "content": "Review this code for issues"},
        ]
        selected = engine.select_model(messages)
        assert selected.name == "large/model"

    def test_migration_upgrades_from_local_to_cloud(self):
        local = _make_model(
            name="local/model",
            cost_per_1k_input=0.0,
            is_local=True,
        )
        cloud = _make_model(
            name="cloud/model",
            cost_per_1k_input=0.01,
            is_local=False,
        )
        engine = _make_engine([local, cloud])

        messages = [
            {"role": "user", "content": "Migrate this from Python 2 to Python 3"},
        ]
        selected = engine.select_model(messages)
        assert selected.name == "cloud/model"

    def test_debugging_stays_on_primary(self):
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001)
        expensive = _make_model(name="expensive/model", cost_per_1k_input=0.03)
        engine = _make_engine([cheap, expensive])

        messages = [
            {"role": "user", "content": "Fix this error in my code"},
        ]
        selected = engine.select_model(messages)
        assert selected.name == "cheap/model"

    def test_falls_back_to_primary_when_no_model_meets_requirements(self):
        small = _make_model(
            name="small/model",
            cost_per_1k_input=0.001,
            max_context_window=4000,
        )
        engine = _make_engine([small])

        messages = [
            {"role": "user", "content": "Refactor this entire codebase"},
        ]
        selected = engine.select_model(messages)
        assert selected.name == "small/model"

    def test_picks_cheapest_among_qualifying_when_primary_unfit(self):
        small = _make_model(
            name="small/model",
            cost_per_1k_input=0.001,
            max_context_window=8000,
        )
        large_cheap = _make_model(
            name="large_cheap/model",
            cost_per_1k_input=0.005,
            max_context_window=64000,
        )
        large_expensive = _make_model(
            name="large_expensive/model",
            cost_per_1k_input=0.03,
            max_context_window=128000,
        )
        engine = _make_engine([small, large_cheap, large_expensive])

        messages = [
            {"role": "user", "content": "Write tests for this service"},
        ]
        selected = engine.select_model(messages)
        assert selected.name == "large_cheap/model"

    def test_general_task_stays_on_primary(self):
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001)
        expensive = _make_model(name="expensive/model", cost_per_1k_input=0.03)
        engine = _make_engine([cheap, expensive])

        messages = [
            {"role": "user", "content": "Tell me a joke"},
        ]
        selected = engine.select_model(messages)
        assert selected.name == "cheap/model"

    def test_explicit_primary_respected_when_it_meets_requirements(self):
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001)
        expensive = _make_model(
            name="expensive/model",
            cost_per_1k_input=0.03,
            max_context_window=128000,
        )
        engine = _make_engine([cheap, expensive], primary_model="expensive/model")

        messages = [
            {"role": "user", "content": "Refactor this module"},
        ]
        selected = engine.select_model(messages)
        assert selected.name == "expensive/model"


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

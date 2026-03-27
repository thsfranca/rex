from __future__ import annotations

from app.config import ModelConfig, RoutingConfig
from app.router.engine import RoutingEngine
from app.router.registry import ModelRegistry


def _make_model(**overrides) -> ModelConfig:
    defaults = {
        "name": "test/model",
        "provider": "test",
        "context_window": 4096,
        "cost_per_1k_input": 0.001,
        "cost_per_1k_output": 0.002,
        "strengths": ["general"],
        "max_latency_ms": 500,
        "is_local": False,
    }
    defaults.update(overrides)
    return ModelConfig(**defaults)


def _make_engine(
    models: list[ModelConfig],
    completion_model: str,
    default_model: str,
) -> RoutingEngine:
    registry = ModelRegistry(models)
    routing = RoutingConfig(completion_model=completion_model, default_model=default_model)
    return RoutingEngine(registry, routing)


class TestSelectModel:
    def test_short_single_turn_routes_to_completion_model(self):
        fast = _make_model(name="fast/model", max_latency_ms=50)
        strong = _make_model(name="strong/model", max_latency_ms=2000)
        engine = _make_engine([fast, strong], "fast/model", "strong/model")

        messages = [{"role": "user", "content": "def hello"}]
        selected = engine.select_model(messages, max_tokens=50, temperature=0)
        assert selected.name == "fast/model"

    def test_multi_turn_routes_to_default_model(self):
        fast = _make_model(name="fast/model")
        strong = _make_model(name="strong/model")
        engine = _make_engine([fast, strong], "fast/model", "strong/model")

        messages = [
            {"role": "system", "content": "You are a helpful assistant"},
            {"role": "user", "content": "Explain async"},
            {"role": "assistant", "content": "..."},
            {"role": "user", "content": "Show example"},
        ]
        selected = engine.select_model(messages)
        assert selected.name == "strong/model"

    def test_same_model_for_both_routes(self):
        model = _make_model(name="only/model")
        engine = _make_engine([model], "only/model", "only/model")

        messages = [{"role": "user", "content": "short"}]
        selected = engine.select_model(messages, max_tokens=10, temperature=0)
        assert selected.name == "only/model"


class TestFallbackOrder:
    def test_fallback_starts_with_default_when_primary_differs(self):
        fast = _make_model(name="fast/model")
        strong = _make_model(name="strong/model")
        other = _make_model(name="other/model")
        engine = _make_engine([fast, strong, other], "fast/model", "strong/model")

        fallbacks = engine.fallback_order(fast)
        assert fallbacks[0].name == "strong/model"
        assert _make_model(name="other/model").name in [f.name for f in fallbacks]

    def test_fallback_excludes_primary(self):
        fast = _make_model(name="fast/model")
        strong = _make_model(name="strong/model")
        engine = _make_engine([fast, strong], "fast/model", "strong/model")

        fallbacks = engine.fallback_order(fast)
        assert all(f.name != "fast/model" for f in fallbacks)

    def test_fallback_when_primary_is_default(self):
        fast = _make_model(name="fast/model")
        strong = _make_model(name="strong/model")
        engine = _make_engine([fast, strong], "fast/model", "strong/model")

        fallbacks = engine.fallback_order(strong)
        assert len(fallbacks) == 1
        assert fallbacks[0].name == "fast/model"

    def test_fallback_single_model_returns_empty(self):
        model = _make_model(name="only/model")
        engine = _make_engine([model], "only/model", "only/model")

        fallbacks = engine.fallback_order(model)
        assert fallbacks == []

    def test_fallback_no_duplicates(self):
        fast = _make_model(name="fast/model")
        strong = _make_model(name="strong/model")
        third = _make_model(name="third/model")
        engine = _make_engine([fast, strong, third], "fast/model", "strong/model")

        fallbacks = engine.fallback_order(fast)
        names = [f.name for f in fallbacks]
        assert len(names) == len(set(names))

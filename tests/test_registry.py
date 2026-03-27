from __future__ import annotations

from app.config import ModelConfig
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


class TestModelRegistry:
    def test_get_by_name_found(self):
        model = _make_model(name="openai/gpt-4o")
        registry = ModelRegistry([model])
        assert registry.get_by_name("openai/gpt-4o") is model

    def test_get_by_name_not_found(self):
        registry = ModelRegistry([_make_model()])
        assert registry.get_by_name("nonexistent") is None

    def test_get_all(self):
        models = [_make_model(name="a"), _make_model(name="b")]
        registry = ModelRegistry(models)
        assert len(registry.get_all()) == 2

    def test_get_by_strength(self):
        fast = _make_model(name="fast", strengths=["completion"])
        smart = _make_model(name="smart", strengths=["debugging", "generation"])
        registry = ModelRegistry([fast, smart])

        assert registry.get_by_strength("completion") == [fast]
        assert registry.get_by_strength("debugging") == [smart]
        assert registry.get_by_strength("nonexistent") == []

    def test_names(self):
        models = [_make_model(name="a"), _make_model(name="b")]
        registry = ModelRegistry(models)
        assert registry.names() == ["a", "b"]

    def test_empty_registry(self):
        registry = ModelRegistry([])
        assert registry.get_all() == []
        assert registry.names() == []
        assert registry.get_by_name("anything") is None

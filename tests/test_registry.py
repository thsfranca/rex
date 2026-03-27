from __future__ import annotations

from app.config import ModelConfig
from app.router.registry import ModelRegistry


def _make_model(**overrides) -> ModelConfig:
    defaults = {"name": "test/model"}
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

    def test_names(self):
        models = [_make_model(name="a"), _make_model(name="b")]
        registry = ModelRegistry(models)
        assert registry.names() == ["a", "b"]

    def test_empty_registry(self):
        registry = ModelRegistry([])
        assert registry.get_all() == []
        assert registry.names() == []
        assert registry.get_by_name("anything") is None


class TestSortedByCost:
    def test_local_models_first(self):
        cloud = _make_model(name="cloud", cost_per_1k_input=0.01, is_local=False)
        local = _make_model(name="local", cost_per_1k_input=0.0, is_local=True)
        registry = ModelRegistry([cloud, local])

        sorted_models = registry.sorted_by_cost()
        assert sorted_models[0].name == "local"
        assert sorted_models[1].name == "cloud"

    def test_cheaper_cloud_before_expensive_cloud(self):
        expensive = _make_model(name="expensive", cost_per_1k_input=0.03)
        cheap = _make_model(name="cheap", cost_per_1k_input=0.001)
        registry = ModelRegistry([expensive, cheap])

        sorted_models = registry.sorted_by_cost()
        assert sorted_models[0].name == "cheap"
        assert sorted_models[1].name == "expensive"

    def test_empty_registry_returns_empty(self):
        registry = ModelRegistry([])
        assert registry.sorted_by_cost() == []

    def test_single_model(self):
        model = _make_model(name="only")
        registry = ModelRegistry([model])
        assert registry.sorted_by_cost() == [model]

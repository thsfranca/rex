from __future__ import annotations

from app.config import Model
from app.router.categories import TaskRequirements
from app.router.registry import ModelRegistry


def _make_model(**overrides) -> Model:
    defaults = {"name": "test/model"}
    defaults.update(overrides)
    return Model(**defaults)


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


class TestFilterByRequirements:
    def test_no_requirements_returns_all_sorted(self):
        cheap = _make_model(name="cheap", cost_per_1k_input=0.001)
        expensive = _make_model(name="expensive", cost_per_1k_input=0.03)
        registry = ModelRegistry([expensive, cheap])

        result = registry.filter_by_requirements(TaskRequirements())
        assert [m.name for m in result] == ["cheap", "expensive"]

    def test_filters_by_min_context_window(self):
        small = _make_model(name="small", max_context_window=8000, cost_per_1k_input=0.001)
        large = _make_model(name="large", max_context_window=128000, cost_per_1k_input=0.01)
        registry = ModelRegistry([small, large])

        req = TaskRequirements(min_context_window=32_000)
        result = registry.filter_by_requirements(req)
        assert [m.name for m in result] == ["large"]

    def test_excludes_unknown_context_window(self):
        unknown = _make_model(name="unknown", max_context_window=None, cost_per_1k_input=0.001)
        known = _make_model(name="known", max_context_window=64000, cost_per_1k_input=0.01)
        registry = ModelRegistry([unknown, known])

        req = TaskRequirements(min_context_window=16_000)
        result = registry.filter_by_requirements(req)
        assert [m.name for m in result] == ["known"]

    def test_filters_by_function_calling(self):
        no_fc = _make_model(
            name="no_fc",
            supports_function_calling=False,
            cost_per_1k_input=0.001,
        )
        has_fc = _make_model(
            name="has_fc",
            supports_function_calling=True,
            cost_per_1k_input=0.01,
        )
        registry = ModelRegistry([no_fc, has_fc])

        req = TaskRequirements(needs_function_calling=True)
        result = registry.filter_by_requirements(req)
        assert [m.name for m in result] == ["has_fc"]

    def test_filters_by_reasoning(self):
        no_reason = _make_model(
            name="no_reason",
            supports_reasoning=False,
            cost_per_1k_input=0.001,
        )
        has_reason = _make_model(
            name="has_reason",
            supports_reasoning=True,
            cost_per_1k_input=0.01,
        )
        registry = ModelRegistry([no_reason, has_reason])

        req = TaskRequirements(needs_reasoning=True)
        result = registry.filter_by_requirements(req)
        assert [m.name for m in result] == ["has_reason"]

    def test_filters_by_cloud_requirement(self):
        local = _make_model(name="local", is_local=True, cost_per_1k_input=0.0)
        cloud = _make_model(name="cloud", is_local=False, cost_per_1k_input=0.01)
        registry = ModelRegistry([local, cloud])

        req = TaskRequirements(needs_cloud=True)
        result = registry.filter_by_requirements(req)
        assert [m.name for m in result] == ["cloud"]

    def test_combines_multiple_requirements(self):
        small_cloud = _make_model(
            name="small_cloud",
            max_context_window=8000,
            is_local=False,
            cost_per_1k_input=0.001,
        )
        large_local = _make_model(
            name="large_local",
            max_context_window=128000,
            is_local=True,
            cost_per_1k_input=0.0,
        )
        large_cloud = _make_model(
            name="large_cloud",
            max_context_window=128000,
            is_local=False,
            cost_per_1k_input=0.01,
        )
        registry = ModelRegistry([small_cloud, large_local, large_cloud])

        req = TaskRequirements(min_context_window=32_000, needs_cloud=True)
        result = registry.filter_by_requirements(req)
        assert [m.name for m in result] == ["large_cloud"]

    def test_returns_empty_when_no_match(self):
        model = _make_model(name="small", max_context_window=4000, cost_per_1k_input=0.001)
        registry = ModelRegistry([model])

        req = TaskRequirements(min_context_window=128_000)
        result = registry.filter_by_requirements(req)
        assert result == []

    def test_meets_requirements_passes(self):
        model = _make_model(
            name="capable",
            max_context_window=128000,
            supports_reasoning=True,
            supports_function_calling=True,
            is_local=False,
        )
        registry = ModelRegistry([model])
        req = TaskRequirements(
            min_context_window=32_000,
            needs_reasoning=True,
            needs_function_calling=True,
        )
        assert registry.meets_requirements(model, req) is True

    def test_meets_requirements_fails_context_window(self):
        model = _make_model(name="small", max_context_window=8000)
        registry = ModelRegistry([model])
        req = TaskRequirements(min_context_window=32_000)
        assert registry.meets_requirements(model, req) is False

    def test_meets_requirements_fails_reasoning(self):
        model = _make_model(name="basic", supports_reasoning=False)
        registry = ModelRegistry([model])
        req = TaskRequirements(needs_reasoning=True)
        assert registry.meets_requirements(model, req) is False

    def test_meets_requirements_fails_cloud(self):
        model = _make_model(name="local", is_local=True)
        registry = ModelRegistry([model])
        req = TaskRequirements(needs_cloud=True)
        assert registry.meets_requirements(model, req) is False

    def test_results_sorted_by_cost(self):
        expensive = _make_model(
            name="expensive",
            max_context_window=128000,
            cost_per_1k_input=0.03,
        )
        cheap = _make_model(
            name="cheap",
            max_context_window=128000,
            cost_per_1k_input=0.001,
        )
        local = _make_model(
            name="local",
            max_context_window=128000,
            is_local=True,
            cost_per_1k_input=0.0,
        )
        registry = ModelRegistry([expensive, cheap, local])

        req = TaskRequirements(min_context_window=32_000)
        result = registry.filter_by_requirements(req)
        assert [m.name for m in result] == ["local", "cheap", "expensive"]

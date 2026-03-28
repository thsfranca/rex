from __future__ import annotations

from app.config import Model
from app.router.categories import TaskRequirements


class ModelRegistry:
    def __init__(self, models: list[Model]) -> None:
        self._models = {m.name: m for m in models}

    def get_by_name(self, name: str) -> Model | None:
        return self._models.get(name)

    def get_all(self) -> list[Model]:
        return list(self._models.values())

    def sorted_by_cost(self) -> list[Model]:
        return sorted(
            self._models.values(),
            key=lambda m: (not m.is_local, m.cost_per_1k_input),
        )

    @staticmethod
    def meets_requirements(model: Model, requirements: TaskRequirements) -> bool:
        if requirements.min_context_window is not None:
            if (
                model.max_context_window is None
                or model.max_context_window < requirements.min_context_window
            ):
                return False
        if requirements.needs_function_calling and not model.supports_function_calling:
            return False
        if requirements.needs_reasoning and not model.supports_reasoning:
            return False
        if requirements.needs_cloud and model.is_local:
            return False
        return True

    def filter_by_requirements(self, requirements: TaskRequirements) -> list[Model]:
        candidates = [m for m in self._models.values() if self.meets_requirements(m, requirements)]
        return sorted(
            candidates,
            key=lambda m: (not m.is_local, m.cost_per_1k_input),
        )

    def names(self) -> list[str]:
        return list(self._models.keys())

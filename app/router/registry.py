from __future__ import annotations

from app.config import ModelConfig
from app.router.categories import TaskRequirements


class ModelRegistry:
    def __init__(self, models: list[ModelConfig]) -> None:
        self._models = {m.name: m for m in models}

    def get_by_name(self, name: str) -> ModelConfig | None:
        return self._models.get(name)

    def get_all(self) -> list[ModelConfig]:
        return list(self._models.values())

    def sorted_by_cost(self) -> list[ModelConfig]:
        return sorted(
            self._models.values(),
            key=lambda m: (not m.is_local, m.cost_per_1k_input),
        )

    def filter_by_requirements(self, requirements: TaskRequirements) -> list[ModelConfig]:
        candidates = []
        for m in self._models.values():
            if requirements.min_context_window is not None:
                if (
                    m.max_context_window is None
                    or m.max_context_window < requirements.min_context_window
                ):
                    continue
            if requirements.needs_function_calling:
                if not m.supports_function_calling:
                    continue
            if requirements.needs_cloud:
                if m.is_local:
                    continue
            candidates.append(m)
        return sorted(
            candidates,
            key=lambda m: (not m.is_local, m.cost_per_1k_input),
        )

    def names(self) -> list[str]:
        return list(self._models.keys())

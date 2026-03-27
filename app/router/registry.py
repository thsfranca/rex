from __future__ import annotations

from app.config import ModelConfig


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

    def names(self) -> list[str]:
        return list(self._models.keys())

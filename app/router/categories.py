from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


class TaskCategory(str, Enum):
    COMPLETION = "completion"
    DEBUGGING = "debugging"
    REFACTORING = "refactoring"
    OPTIMIZATION = "optimization"
    TEST_GENERATION = "test_generation"
    EXPLANATION = "explanation"
    DOCUMENTATION = "documentation"
    CODE_REVIEW = "code_review"
    GENERATION = "generation"
    MIGRATION = "migration"
    GENERAL = "general"


@dataclass(frozen=True)
class TaskRequirements:
    min_context_window: int | None = None
    needs_function_calling: bool = False
    needs_reasoning: bool = False
    needs_cloud: bool = False


CATEGORY_REQUIREMENTS: dict[TaskCategory, TaskRequirements] = {
    TaskCategory.COMPLETION: TaskRequirements(),
    TaskCategory.DEBUGGING: TaskRequirements(needs_reasoning=True),
    TaskCategory.REFACTORING: TaskRequirements(min_context_window=32_000),
    TaskCategory.OPTIMIZATION: TaskRequirements(needs_reasoning=True),
    TaskCategory.TEST_GENERATION: TaskRequirements(min_context_window=16_000),
    TaskCategory.EXPLANATION: TaskRequirements(),
    TaskCategory.DOCUMENTATION: TaskRequirements(min_context_window=16_000),
    TaskCategory.CODE_REVIEW: TaskRequirements(min_context_window=32_000, needs_reasoning=True),
    TaskCategory.GENERATION: TaskRequirements(min_context_window=16_000),
    TaskCategory.MIGRATION: TaskRequirements(needs_reasoning=True, min_context_window=32_000),
    TaskCategory.GENERAL: TaskRequirements(),
}


def get_requirements(category: TaskCategory) -> TaskRequirements:
    return CATEGORY_REQUIREMENTS[category]

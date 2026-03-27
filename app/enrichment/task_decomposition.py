from __future__ import annotations

from app.enrichment.context import EnrichmentContext
from app.router.categories import TaskCategory
from app.router.detector import FeatureType

COMPLEX_CATEGORIES: frozenset[TaskCategory] = frozenset(
    {
        TaskCategory.GENERATION,
        TaskCategory.REFACTORING,
        TaskCategory.MIGRATION,
        TaskCategory.CODE_REVIEW,
        TaskCategory.TEST_GENERATION,
        TaskCategory.DOCUMENTATION,
    }
)

DECOMPOSITION_INSTRUCTION = (
    "Before starting, break this task into a numbered list of concrete subtasks. "
    "Work through each subtask one at a time. "
    "State which subtask you are working on and mark it done before moving to the next."
)


def _is_complex(context: EnrichmentContext) -> bool:
    if context.feature_type != FeatureType.CHAT:
        return False
    return context.category in COMPLEX_CATEGORIES


class TaskDecompositionEnricher:
    def enrich(self, context: EnrichmentContext) -> EnrichmentContext:
        if not _is_complex(context):
            return context

        for msg in context.messages:
            if msg.get("role") == "system":
                msg["content"] = msg.get("content", "") + "\n\n" + DECOMPOSITION_INSTRUCTION
                return context

        context.messages.insert(0, {"role": "system", "content": DECOMPOSITION_INSTRUCTION})
        return context

from __future__ import annotations

from app.enrichment.context import EnrichmentContext
from app.enrichment.task_decomposition import (
    COMPLEX_CATEGORIES,
    DECOMPOSITION_INSTRUCTION,
    PROMPT_LENGTH_THRESHOLD,
    TaskDecompositionEnricher,
)
from app.router.categories import TaskCategory
from app.router.detector import FeatureType


def _make_context(
    messages: list[dict] | None = None,
    category: TaskCategory = TaskCategory.GENERATION,
    confidence: float = 0.8,
    feature_type: FeatureType = FeatureType.CHAT,
) -> EnrichmentContext:
    return EnrichmentContext(
        messages=messages or [{"role": "user", "content": "build something"}],
        category=category,
        confidence=confidence,
        feature_type=feature_type,
    )


class TestComplexCategories:
    def test_generation_is_complex(self):
        assert TaskCategory.GENERATION in COMPLEX_CATEGORIES

    def test_refactoring_is_complex(self):
        assert TaskCategory.REFACTORING in COMPLEX_CATEGORIES

    def test_migration_is_complex(self):
        assert TaskCategory.MIGRATION in COMPLEX_CATEGORIES

    def test_code_review_is_complex(self):
        assert TaskCategory.CODE_REVIEW in COMPLEX_CATEGORIES

    def test_test_generation_is_complex(self):
        assert TaskCategory.TEST_GENERATION in COMPLEX_CATEGORIES

    def test_completion_is_not_complex(self):
        assert TaskCategory.COMPLETION not in COMPLEX_CATEGORIES

    def test_debugging_is_not_complex(self):
        assert TaskCategory.DEBUGGING not in COMPLEX_CATEGORIES

    def test_explanation_is_not_complex(self):
        assert TaskCategory.EXPLANATION not in COMPLEX_CATEGORIES

    def test_documentation_is_complex(self):
        assert TaskCategory.DOCUMENTATION in COMPLEX_CATEGORIES

    def test_general_is_not_complex(self):
        assert TaskCategory.GENERAL not in COMPLEX_CATEGORIES

    def test_optimization_is_not_complex(self):
        assert TaskCategory.OPTIMIZATION not in COMPLEX_CATEGORIES


class TestPromptLengthThreshold:
    def test_threshold_is_500(self):
        assert PROMPT_LENGTH_THRESHOLD == 500


class TestTaskDecompositionEnricher:
    def test_enriches_complex_chat_request(self):
        enricher = TaskDecompositionEnricher()
        ctx = _make_context(category=TaskCategory.GENERATION)
        result = enricher.enrich(ctx)
        assert any(
            DECOMPOSITION_INSTRUCTION in msg.get("content", "")
            for msg in result.messages
            if msg.get("role") == "system"
        )

    def test_skips_completion_requests(self):
        enricher = TaskDecompositionEnricher()
        ctx = _make_context(
            category=TaskCategory.GENERATION,
            feature_type=FeatureType.COMPLETION,
        )
        result = enricher.enrich(ctx)
        assert not any(
            DECOMPOSITION_INSTRUCTION in msg.get("content", "") for msg in result.messages
        )

    def test_skips_simple_categories(self):
        enricher = TaskDecompositionEnricher()
        simple_categories = [
            TaskCategory.COMPLETION,
            TaskCategory.DEBUGGING,
            TaskCategory.OPTIMIZATION,
            TaskCategory.EXPLANATION,
            TaskCategory.GENERAL,
        ]
        for category in simple_categories:
            ctx = _make_context(category=category)
            result = enricher.enrich(ctx)
            assert not any(
                DECOMPOSITION_INSTRUCTION in msg.get("content", "") for msg in result.messages
            ), f"Expected skip for {category.value}"

    def test_appends_to_existing_system_message(self):
        enricher = TaskDecompositionEnricher()
        ctx = _make_context(
            messages=[
                {"role": "system", "content": "You are a helpful assistant"},
                {"role": "user", "content": "build something"},
            ],
            category=TaskCategory.GENERATION,
        )
        result = enricher.enrich(ctx)
        system_msg = result.messages[0]
        assert system_msg["content"].startswith("You are a helpful assistant")
        assert DECOMPOSITION_INSTRUCTION in system_msg["content"]

    def test_creates_system_message_when_none_exists(self):
        enricher = TaskDecompositionEnricher()
        ctx = _make_context(
            messages=[{"role": "user", "content": "build something"}],
            category=TaskCategory.GENERATION,
        )
        result = enricher.enrich(ctx)
        assert result.messages[0]["role"] == "system"
        assert result.messages[0]["content"] == DECOMPOSITION_INSTRUCTION
        assert result.messages[1]["role"] == "user"

    def test_does_not_replace_system_message(self):
        enricher = TaskDecompositionEnricher()
        original_system = "You are a coding assistant"
        ctx = _make_context(
            messages=[
                {"role": "system", "content": original_system},
                {"role": "user", "content": "refactor this"},
            ],
            category=TaskCategory.REFACTORING,
        )
        result = enricher.enrich(ctx)
        assert original_system in result.messages[0]["content"]

    def test_enriches_all_complex_categories(self):
        enricher = TaskDecompositionEnricher()
        for category in COMPLEX_CATEGORIES:
            ctx = _make_context(category=category)
            result = enricher.enrich(ctx)
            assert any(
                DECOMPOSITION_INSTRUCTION in msg.get("content", "") for msg in result.messages
            ), f"Expected enrichment for {category.value}"

    def test_simple_category_with_long_prompt_still_skips(self):
        enricher = TaskDecompositionEnricher()
        long_prompt = "explain " + "x " * 1000
        ctx = _make_context(
            messages=[{"role": "user", "content": long_prompt}],
            category=TaskCategory.EXPLANATION,
        )
        result = enricher.enrich(ctx)
        assert not any(
            DECOMPOSITION_INSTRUCTION in msg.get("content", "") for msg in result.messages
        )

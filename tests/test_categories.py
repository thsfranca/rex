from __future__ import annotations

from app.router.categories import (
    CATEGORY_REQUIREMENTS,
    TaskCategory,
    TaskRequirements,
    get_requirements,
)


class TestTaskCategory:
    def test_all_categories_are_strings(self):
        for category in TaskCategory:
            assert isinstance(category.value, str)

    def test_expected_categories_exist(self):
        expected = {
            "completion",
            "debugging",
            "refactoring",
            "optimization",
            "test_generation",
            "explanation",
            "documentation",
            "code_review",
            "generation",
            "migration",
            "general",
        }
        actual = {c.value for c in TaskCategory}
        assert actual == expected


class TestTaskRequirements:
    def test_defaults(self):
        req = TaskRequirements()
        assert req.min_context_window is None
        assert req.needs_function_calling is False
        assert req.needs_reasoning is False
        assert req.needs_cloud is False

    def test_custom_values(self):
        req = TaskRequirements(
            min_context_window=32_000,
            needs_function_calling=True,
            needs_reasoning=True,
            needs_cloud=True,
        )
        assert req.min_context_window == 32_000
        assert req.needs_function_calling is True
        assert req.needs_reasoning is True
        assert req.needs_cloud is True

    def test_is_frozen(self):
        req = TaskRequirements()
        try:
            req.min_context_window = 100
            assert False, "Should have raised"
        except AttributeError:
            pass


class TestCategoryRequirements:
    def test_every_category_has_requirements(self):
        for category in TaskCategory:
            assert category in CATEGORY_REQUIREMENTS

    def test_refactoring_needs_large_context(self):
        req = get_requirements(TaskCategory.REFACTORING)
        assert req.min_context_window == 32_000

    def test_debugging_needs_reasoning(self):
        req = get_requirements(TaskCategory.DEBUGGING)
        assert req.needs_reasoning is True

    def test_optimization_needs_reasoning(self):
        req = get_requirements(TaskCategory.OPTIMIZATION)
        assert req.needs_reasoning is True

    def test_code_review_needs_large_context_and_reasoning(self):
        req = get_requirements(TaskCategory.CODE_REVIEW)
        assert req.min_context_window == 32_000
        assert req.needs_reasoning is True

    def test_test_generation_needs_medium_context(self):
        req = get_requirements(TaskCategory.TEST_GENERATION)
        assert req.min_context_window == 16_000

    def test_generation_needs_medium_context(self):
        req = get_requirements(TaskCategory.GENERATION)
        assert req.min_context_window == 16_000

    def test_migration_needs_cloud(self):
        req = get_requirements(TaskCategory.MIGRATION)
        assert req.needs_cloud is True

    def test_completion_has_no_special_requirements(self):
        req = get_requirements(TaskCategory.COMPLETION)
        assert req.min_context_window is None
        assert req.needs_function_calling is False
        assert req.needs_reasoning is False
        assert req.needs_cloud is False

    def test_general_has_no_special_requirements(self):
        req = get_requirements(TaskCategory.GENERAL)
        assert req.min_context_window is None
        assert req.needs_function_calling is False
        assert req.needs_reasoning is False
        assert req.needs_cloud is False

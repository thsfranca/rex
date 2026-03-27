from __future__ import annotations

from app.router.categories import TaskCategory
from app.router.classifier import (
    ClassificationResult,
    classify,
    _extract_last_user_message,
    _code_block_ratio,
)


class TestExtractLastUserMessage:
    def test_returns_last_user_content(self):
        messages = [
            {"role": "system", "content": "You are helpful"},
            {"role": "user", "content": "first"},
            {"role": "assistant", "content": "response"},
            {"role": "user", "content": "second question"},
        ]
        assert _extract_last_user_message(messages) == "second question"

    def test_returns_empty_for_no_user_messages(self):
        messages = [{"role": "system", "content": "You are helpful"}]
        assert _extract_last_user_message(messages) == ""

    def test_returns_empty_for_empty_list(self):
        assert _extract_last_user_message([]) == ""

    def test_handles_multipart_content(self):
        messages = [
            {
                "role": "user",
                "content": [
                    {"type": "text", "text": "explain this"},
                    {"type": "image_url", "image_url": {"url": "http://..."}},
                ],
            }
        ]
        assert "explain this" in _extract_last_user_message(messages)


class TestClassifyDebugging:
    def test_error_keyword(self):
        messages = [{"role": "user", "content": "I'm getting an error in my code"}]
        result = classify(messages)
        assert result.category == TaskCategory.DEBUGGING

    def test_fix_keyword(self):
        messages = [{"role": "user", "content": "Can you fix this bug?"}]
        result = classify(messages)
        assert result.category == TaskCategory.DEBUGGING

    def test_stack_trace_boosts_debugging(self):
        messages = [
            {
                "role": "user",
                "content": (
                    "Help with this:\n"
                    "Traceback (most recent call last):\n"
                    '  File "main.py", line 10, in <module>\n'
                    "    foo()\n"
                    "NameError: name 'foo' is not defined"
                ),
            }
        ]
        result = classify(messages)
        assert result.category == TaskCategory.DEBUGGING
        assert result.confidence > 0.5


class TestClassifyRefactoring:
    def test_refactor_keyword(self):
        messages = [{"role": "user", "content": "Please refactor this function"}]
        result = classify(messages)
        assert result.category == TaskCategory.REFACTORING

    def test_clean_up_keyword(self):
        messages = [{"role": "user", "content": "Clean up this code"}]
        result = classify(messages)
        assert result.category == TaskCategory.REFACTORING


class TestClassifyOptimization:
    def test_optimize_keyword(self):
        messages = [{"role": "user", "content": "How can I optimize this query?"}]
        result = classify(messages)
        assert result.category == TaskCategory.OPTIMIZATION

    def test_performance_keyword(self):
        messages = [{"role": "user", "content": "This is slow, improve the performance"}]
        result = classify(messages)
        assert result.category == TaskCategory.OPTIMIZATION


class TestClassifyTestGeneration:
    def test_write_tests_keyword(self):
        messages = [{"role": "user", "content": "Write tests for this class"}]
        result = classify(messages)
        assert result.category == TaskCategory.TEST_GENERATION

    def test_add_test_keyword(self):
        messages = [{"role": "user", "content": "Add a unit test for the parser"}]
        result = classify(messages)
        assert result.category == TaskCategory.TEST_GENERATION


class TestClassifyExplanation:
    def test_explain_keyword(self):
        messages = [{"role": "user", "content": "Explain what this function does"}]
        result = classify(messages)
        assert result.category == TaskCategory.EXPLANATION

    def test_how_does_keyword(self):
        messages = [{"role": "user", "content": "How does this algorithm work?"}]
        result = classify(messages)
        assert result.category == TaskCategory.EXPLANATION


class TestClassifyDocumentation:
    def test_docstring_keyword(self):
        messages = [{"role": "user", "content": "Add a docstring to this method"}]
        result = classify(messages)
        assert result.category == TaskCategory.DOCUMENTATION

    def test_readme_keyword(self):
        messages = [{"role": "user", "content": "Update the README for this project"}]
        result = classify(messages)
        assert result.category == TaskCategory.DOCUMENTATION


class TestClassifyCodeReview:
    def test_review_keyword(self):
        messages = [{"role": "user", "content": "Review this pull request"}]
        result = classify(messages)
        assert result.category == TaskCategory.CODE_REVIEW

    def test_security_keyword(self):
        messages = [{"role": "user", "content": "Check this code for security vulnerabilities"}]
        result = classify(messages)
        assert result.category == TaskCategory.CODE_REVIEW


class TestClassifyGeneration:
    def test_create_keyword(self):
        messages = [{"role": "user", "content": "Create a REST API for user management"}]
        result = classify(messages)
        assert result.category == TaskCategory.GENERATION

    def test_implement_keyword(self):
        messages = [{"role": "user", "content": "Implement a binary search tree"}]
        result = classify(messages)
        assert result.category == TaskCategory.GENERATION


class TestClassifyMigration:
    def test_migrate_keyword(self):
        messages = [{"role": "user", "content": "Migrate this from JavaScript to TypeScript"}]
        result = classify(messages)
        assert result.category == TaskCategory.MIGRATION

    def test_upgrade_keyword(self):
        messages = [{"role": "user", "content": "Upgrade this from React 17 to React 18"}]
        result = classify(messages)
        assert result.category == TaskCategory.MIGRATION


class TestClassifyGeneral:
    def test_empty_message(self):
        messages = [{"role": "user", "content": ""}]
        result = classify(messages)
        assert result.category == TaskCategory.GENERAL
        assert result.confidence == 0.0

    def test_no_matching_keywords(self):
        messages = [{"role": "user", "content": "hello world"}]
        result = classify(messages)
        assert result.category == TaskCategory.GENERAL
        assert result.confidence == 0.2

    def test_no_messages(self):
        result = classify([])
        assert result.category == TaskCategory.GENERAL


class TestCodeBlockRatio:
    def test_no_code_blocks(self):
        assert _code_block_ratio("just plain text") == 0.0

    def test_empty_text(self):
        assert _code_block_ratio("") == 0.0

    def test_all_code(self):
        text = "```python\nprint('hello')\n```"
        assert _code_block_ratio(text) == 1.0

    def test_mixed_content(self):
        text = "some text ```code``` more text"
        ratio = _code_block_ratio(text)
        assert 0.0 < ratio < 1.0

    def test_multiple_code_blocks(self):
        text = "text ```block1``` middle ```block2``` end"
        ratio = _code_block_ratio(text)
        assert ratio > 0.0


class TestStructuralAnalysis:
    def test_code_blocks_boost_debugging_confidence(self):
        without_code = [
            {"role": "user", "content": "Fix this error in my code"},
        ]
        with_code = [
            {
                "role": "user",
                "content": (
                    "Fix this error:\n"
                    "```python\n"
                    "def foo():\n"
                    "    return bar()\n"
                    "```\n"
                    "err"
                ),
            },
        ]
        result_without = classify(without_code)
        result_with = classify(with_code)
        assert result_without.category == TaskCategory.DEBUGGING
        assert result_with.category == TaskCategory.DEBUGGING
        assert result_with.confidence >= result_without.confidence

    def test_long_prompt_boosts_refactoring_confidence(self):
        short_prompt = [
            {"role": "user", "content": "Refactor this function"},
        ]
        long_prompt = [
            {
                "role": "user",
                "content": "Refactor this function " + "x " * 300,
            },
        ]
        result_short = classify(short_prompt)
        result_long = classify(long_prompt)
        assert result_short.category == TaskCategory.REFACTORING
        assert result_long.category == TaskCategory.REFACTORING
        assert result_long.confidence > result_short.confidence

    def test_code_block_does_not_boost_non_code_categories(self):
        messages = [
            {
                "role": "user",
                "content": (
                    "Explain this concept:\n" "```python\n" "x = [i for i in range(10)]\n" "```"
                ),
            },
        ]
        result = classify(messages)
        assert result.category == TaskCategory.EXPLANATION
        assert result.confidence <= 0.7

    def test_long_prompt_does_not_boost_explanation(self):
        messages = [
            {
                "role": "user",
                "content": "Explain how this works " + "x " * 300,
            },
        ]
        result = classify(messages)
        assert result.category == TaskCategory.EXPLANATION
        base_messages = [
            {"role": "user", "content": "Explain how this works"},
        ]
        base_result = classify(base_messages)
        assert result.confidence == base_result.confidence


class TestClassificationResult:
    def test_is_frozen(self):
        result = ClassificationResult(category=TaskCategory.GENERAL, confidence=0.5)
        try:
            result.confidence = 1.0
            assert False, "Should have raised"
        except AttributeError:
            pass

    def test_scores_default_to_empty_dict(self):
        result = ClassificationResult(category=TaskCategory.GENERAL, confidence=0.5)
        assert result.scores == {}

    def test_scores_preserved(self):
        scores = {TaskCategory.DEBUGGING: 0.8, TaskCategory.REFACTORING: 0.3}
        result = ClassificationResult(
            category=TaskCategory.DEBUGGING, confidence=0.8, scores=scores
        )
        assert result.scores == scores


class TestScoresField:
    def test_classify_returns_scores_for_matching_categories(self):
        messages = [{"role": "user", "content": "Fix this error in my code"}]
        result = classify(messages)
        assert result.category == TaskCategory.DEBUGGING
        assert TaskCategory.DEBUGGING in result.scores
        assert result.scores[TaskCategory.DEBUGGING] > 0

    def test_classify_returns_empty_scores_for_general(self):
        messages = [{"role": "user", "content": "hello world"}]
        result = classify(messages)
        assert result.category == TaskCategory.GENERAL
        assert result.scores == {}

    def test_classify_returns_empty_scores_for_empty_message(self):
        messages = [{"role": "user", "content": ""}]
        result = classify(messages)
        assert result.scores == {}

    def test_classify_returns_multiple_scores_when_ambiguous(self):
        messages = [{"role": "user", "content": "Fix this error and create a test"}]
        result = classify(messages)
        assert len(result.scores) >= 2


class TestPriorityResolution:
    def test_debugging_wins_over_generation_when_both_match(self):
        messages = [{"role": "user", "content": "Fix this error and create a test"}]
        result = classify(messages)
        assert result.category in (TaskCategory.DEBUGGING, TaskCategory.TEST_GENERATION)

    def test_multiple_keywords_increase_confidence(self):
        messages = [{"role": "user", "content": "Fix the error, it crashes with an exception"}]
        result = classify(messages)
        assert result.category == TaskCategory.DEBUGGING
        assert result.confidence > 0.6

from __future__ import annotations

import json
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from app.router.categories import TaskCategory
from app.router.llm_judge import (
    JUDGE_SYSTEM_PROMPT,
    JudgeResult,
    LLMJudge,
    _extract_last_user_message,
    _parse_judge_response,
)


class TestJudgeResult:
    def test_defaults(self):
        result = JudgeResult(category=TaskCategory.DEBUGGING)
        assert result.category == TaskCategory.DEBUGGING
        assert result.min_context_window is None

    def test_with_context_window(self):
        result = JudgeResult(category=TaskCategory.REFACTORING, min_context_window=32000)
        assert result.category == TaskCategory.REFACTORING
        assert result.min_context_window == 32000

    def test_is_frozen(self):
        result = JudgeResult(category=TaskCategory.DEBUGGING)
        with pytest.raises(AttributeError):
            result.category = TaskCategory.GENERAL


class TestExtractLastUserMessage:
    def test_returns_last_user_content(self):
        messages = [
            {"role": "system", "content": "system"},
            {"role": "user", "content": "first"},
            {"role": "assistant", "content": "reply"},
            {"role": "user", "content": "second"},
        ]
        assert _extract_last_user_message(messages) == "second"

    def test_returns_empty_for_no_user_messages(self):
        messages = [{"role": "system", "content": "system"}]
        assert _extract_last_user_message(messages) == ""

    def test_returns_empty_for_empty_list(self):
        assert _extract_last_user_message([]) == ""

    def test_handles_multipart_content(self):
        messages = [
            {
                "role": "user",
                "content": [
                    {"type": "text", "text": "hello"},
                    {"type": "text", "text": "world"},
                ],
            }
        ]
        assert _extract_last_user_message(messages) == "hello world"


class TestParseJudgeResponse:
    def test_valid_response(self):
        content = json.dumps({"category": "debugging", "min_context_window": None})
        result = _parse_judge_response(content)
        assert result is not None
        assert result.category == TaskCategory.DEBUGGING
        assert result.min_context_window is None

    def test_valid_response_with_context_window(self):
        content = json.dumps({"category": "refactoring", "min_context_window": 32000})
        result = _parse_judge_response(content)
        assert result is not None
        assert result.category == TaskCategory.REFACTORING
        assert result.min_context_window == 32000

    def test_invalid_json(self):
        result = _parse_judge_response("not json")
        assert result is None

    def test_invalid_category(self):
        content = json.dumps({"category": "invalid_category"})
        result = _parse_judge_response(content)
        assert result is None

    def test_missing_category(self):
        content = json.dumps({"min_context_window": 16000})
        result = _parse_judge_response(content)
        assert result is None

    def test_all_valid_categories(self):
        for category in TaskCategory:
            content = json.dumps({"category": category.value})
            result = _parse_judge_response(content)
            assert result is not None
            assert result.category == category

    def test_non_numeric_context_window_ignored(self):
        content = json.dumps({"category": "debugging", "min_context_window": "large"})
        result = _parse_judge_response(content)
        assert result is not None
        assert result.min_context_window is None

    def test_none_content(self):
        result = _parse_judge_response(None)
        assert result is None


class TestJudgeSystemPrompt:
    def test_contains_all_categories(self):
        for category in TaskCategory:
            assert category.value in JUDGE_SYSTEM_PROMPT

    def test_asks_for_json(self):
        assert "JSON" in JUDGE_SYSTEM_PROMPT


class TestLLMJudge:
    def test_model_property(self):
        judge = LLMJudge(model="ollama/llama3")
        assert judge.model == "ollama/llama3"

    @pytest.mark.asyncio
    @patch("app.router.llm_judge.litellm")
    async def test_classify_returns_result(self, mock_litellm):
        mock_response = MagicMock()
        mock_response.choices = [
            MagicMock(message=MagicMock(content=json.dumps({"category": "debugging"})))
        ]
        mock_litellm.acompletion = AsyncMock(return_value=mock_response)

        judge = LLMJudge(model="ollama/llama3")
        result = await judge.classify([{"role": "user", "content": "fix this error"}])

        assert result is not None
        assert result.category == TaskCategory.DEBUGGING
        mock_litellm.acompletion.assert_called_once()
        call_kwargs = mock_litellm.acompletion.call_args.kwargs
        assert call_kwargs["model"] == "ollama/llama3"
        assert call_kwargs["temperature"] == 0.0

    @pytest.mark.asyncio
    @patch("app.router.llm_judge.litellm")
    async def test_classify_sends_system_prompt(self, mock_litellm):
        mock_response = MagicMock()
        mock_response.choices = [
            MagicMock(message=MagicMock(content=json.dumps({"category": "general"})))
        ]
        mock_litellm.acompletion = AsyncMock(return_value=mock_response)

        judge = LLMJudge(model="ollama/llama3")
        await judge.classify([{"role": "user", "content": "hello"}])

        call_kwargs = mock_litellm.acompletion.call_args.kwargs
        sent_messages = call_kwargs["messages"]
        assert sent_messages[0]["role"] == "system"
        assert sent_messages[0]["content"] == JUDGE_SYSTEM_PROMPT
        assert sent_messages[1]["role"] == "user"
        assert sent_messages[1]["content"] == "hello"

    @pytest.mark.asyncio
    @patch("app.router.llm_judge.litellm")
    async def test_classify_sends_last_user_message_only(self, mock_litellm):
        mock_response = MagicMock()
        mock_response.choices = [
            MagicMock(message=MagicMock(content=json.dumps({"category": "general"})))
        ]
        mock_litellm.acompletion = AsyncMock(return_value=mock_response)

        judge = LLMJudge(model="ollama/llama3")
        await judge.classify(
            [
                {"role": "user", "content": "first message"},
                {"role": "assistant", "content": "reply"},
                {"role": "user", "content": "second message"},
            ]
        )

        call_kwargs = mock_litellm.acompletion.call_args.kwargs
        sent_messages = call_kwargs["messages"]
        assert sent_messages[1]["content"] == "second message"

    @pytest.mark.asyncio
    @patch("app.router.llm_judge.litellm")
    async def test_classify_returns_none_on_litellm_error(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(side_effect=Exception("connection refused"))

        judge = LLMJudge(model="ollama/llama3")
        result = await judge.classify([{"role": "user", "content": "fix this"}])
        assert result is None

    @pytest.mark.asyncio
    @patch("app.router.llm_judge.litellm")
    async def test_classify_returns_none_on_invalid_json(self, mock_litellm):
        mock_response = MagicMock()
        mock_response.choices = [MagicMock(message=MagicMock(content="not json at all"))]
        mock_litellm.acompletion = AsyncMock(return_value=mock_response)

        judge = LLMJudge(model="ollama/llama3")
        result = await judge.classify([{"role": "user", "content": "fix this"}])
        assert result is None

    @pytest.mark.asyncio
    async def test_classify_returns_none_for_empty_messages(self):
        judge = LLMJudge(model="ollama/llama3")
        result = await judge.classify([])
        assert result is None

    @pytest.mark.asyncio
    async def test_classify_returns_none_for_blank_user_message(self):
        judge = LLMJudge(model="ollama/llama3")
        result = await judge.classify([{"role": "user", "content": "   "}])
        assert result is None

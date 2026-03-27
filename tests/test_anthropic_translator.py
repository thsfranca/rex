from __future__ import annotations

from unittest.mock import MagicMock

from app.proxy.anthropic import (
    anthropic_to_openai,
    extract_anthropic_api_key,
    openai_response_to_anthropic,
)


class TestExtractAnthropicApiKey:
    def test_extracts_key_from_header(self):
        request = MagicMock()
        request.headers = {"x-api-key": "sk-ant-test-key"}
        assert extract_anthropic_api_key(request) == "sk-ant-test-key"

    def test_returns_none_when_missing(self):
        request = MagicMock()
        request.headers = {}
        assert extract_anthropic_api_key(request) is None


class TestAnthropicToOpenai:
    def test_simple_user_message(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hello"}],
        }
        result = anthropic_to_openai(body)
        assert result["messages"] == [{"role": "user", "content": "Hello"}]
        assert result["max_tokens"] == 1024

    def test_system_string_becomes_system_message(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "system": "You are helpful.",
            "messages": [{"role": "user", "content": "Hi"}],
        }
        result = anthropic_to_openai(body)
        assert result["messages"][0] == {"role": "system", "content": "You are helpful."}
        assert result["messages"][1] == {"role": "user", "content": "Hi"}

    def test_system_content_blocks(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "system": [
                {"type": "text", "text": "First instruction."},
                {"type": "text", "text": "Second instruction."},
            ],
            "messages": [{"role": "user", "content": "Hi"}],
        }
        result = anthropic_to_openai(body)
        assert result["messages"][0] == {
            "role": "system",
            "content": "First instruction.\nSecond instruction.",
        }

    def test_content_blocks_in_messages(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {"type": "text", "text": "Look at this:"},
                        {"type": "text", "text": "What do you think?"},
                    ],
                }
            ],
        }
        result = anthropic_to_openai(body)
        assert result["messages"][0]["content"] == "Look at this:\nWhat do you think?"

    def test_multi_turn_conversation(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "Hello"},
                {"role": "assistant", "content": "Hi there!"},
                {"role": "user", "content": "How are you?"},
            ],
        }
        result = anthropic_to_openai(body)
        assert len(result["messages"]) == 3
        assert result["messages"][0]["role"] == "user"
        assert result["messages"][1]["role"] == "assistant"
        assert result["messages"][2]["role"] == "user"

    def test_maps_stop_sequences_to_stop(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "stop_sequences": ["\n\nHuman:"],
            "messages": [{"role": "user", "content": "Hi"}],
        }
        result = anthropic_to_openai(body)
        assert result["stop"] == ["\n\nHuman:"]

    def test_passes_temperature(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "temperature": 0.7,
            "messages": [{"role": "user", "content": "Hi"}],
        }
        result = anthropic_to_openai(body)
        assert result["temperature"] == 0.7

    def test_passes_top_p(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "top_p": 0.9,
            "messages": [{"role": "user", "content": "Hi"}],
        }
        result = anthropic_to_openai(body)
        assert result["top_p"] == 0.9

    def test_no_system_prompt(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hi"}],
        }
        result = anthropic_to_openai(body)
        assert len(result["messages"]) == 1
        assert result["messages"][0]["role"] == "user"

    def test_empty_messages(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [],
        }
        result = anthropic_to_openai(body)
        assert result["messages"] == []

    def test_model_not_included_in_output(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hi"}],
        }
        result = anthropic_to_openai(body)
        assert "model" not in result

    def test_non_text_content_blocks_ignored(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {"type": "text", "text": "Hello"},
                        {"type": "image", "source": {"type": "base64", "data": "..."}},
                    ],
                }
            ],
        }
        result = anthropic_to_openai(body)
        assert result["messages"][0]["content"] == "Hello"


class TestOpenaiResponseToAnthropic:
    def _make_response(
        self,
        content="Hello!",
        finish_reason="stop",
        prompt_tokens=10,
        completion_tokens=5,
    ):
        choice = MagicMock()
        choice.message.content = content
        choice.finish_reason = finish_reason

        usage = MagicMock()
        usage.prompt_tokens = prompt_tokens
        usage.completion_tokens = completion_tokens

        response = MagicMock()
        response.choices = [choice]
        response.usage = usage
        return response

    def test_basic_response(self):
        response = self._make_response(content="Hello!")
        result = openai_response_to_anthropic(response, "ollama/llama3")
        assert result["type"] == "message"
        assert result["role"] == "assistant"
        assert result["content"] == [{"type": "text", "text": "Hello!"}]
        assert result["stop_reason"] == "end_turn"
        assert result["stop_sequence"] is None

    def test_uses_request_model_when_provided(self):
        response = self._make_response()
        result = openai_response_to_anthropic(
            response, "ollama/llama3", request_model="claude-3-sonnet"
        )
        assert result["model"] == "claude-3-sonnet"

    def test_uses_model_name_when_no_request_model(self):
        response = self._make_response()
        result = openai_response_to_anthropic(response, "ollama/llama3")
        assert result["model"] == "ollama/llama3"

    def test_maps_stop_to_end_turn(self):
        response = self._make_response(finish_reason="stop")
        result = openai_response_to_anthropic(response, "ollama/llama3")
        assert result["stop_reason"] == "end_turn"

    def test_maps_length_to_max_tokens(self):
        response = self._make_response(finish_reason="length")
        result = openai_response_to_anthropic(response, "ollama/llama3")
        assert result["stop_reason"] == "max_tokens"

    def test_unknown_finish_reason_defaults_to_end_turn(self):
        response = self._make_response(finish_reason="content_filter")
        result = openai_response_to_anthropic(response, "ollama/llama3")
        assert result["stop_reason"] == "end_turn"

    def test_usage_tokens(self):
        response = self._make_response(prompt_tokens=100, completion_tokens=50)
        result = openai_response_to_anthropic(response, "ollama/llama3")
        assert result["usage"] == {"input_tokens": 100, "output_tokens": 50}

    def test_id_format(self):
        response = self._make_response()
        result = openai_response_to_anthropic(response, "ollama/llama3")
        assert result["id"].startswith("msg_")
        assert len(result["id"]) == 28

    def test_no_choices(self):
        response = MagicMock()
        response.choices = []
        response.usage = None
        result = openai_response_to_anthropic(response, "ollama/llama3")
        assert result["content"] == [{"type": "text", "text": ""}]
        assert result["stop_reason"] == "end_turn"

    def test_null_usage(self):
        response = self._make_response()
        response.usage = None
        result = openai_response_to_anthropic(response, "ollama/llama3")
        assert result["usage"] == {"input_tokens": 0, "output_tokens": 0}

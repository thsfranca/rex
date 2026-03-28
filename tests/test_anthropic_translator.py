from __future__ import annotations

import json
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

    def test_image_block_converted_to_openai_image_url(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {"type": "text", "text": "What is this?"},
                        {
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": "image/png",
                                "data": "abc123",
                            },
                        },
                    ],
                }
            ],
        }
        result = anthropic_to_openai(body)
        content = result["messages"][0]["content"]
        assert isinstance(content, list)
        assert content[0] == {"type": "text", "text": "What is this?"}
        assert content[1] == {
            "type": "image_url",
            "image_url": {"url": "data:image/png;base64,abc123"},
        }

    def test_image_url_source_converted(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {"type": "text", "text": "Describe"},
                        {
                            "type": "image",
                            "source": {
                                "type": "url",
                                "url": "https://example.com/img.png",
                            },
                        },
                    ],
                }
            ],
        }
        result = anthropic_to_openai(body)
        content = result["messages"][0]["content"]
        assert isinstance(content, list)
        assert content[1] == {
            "type": "image_url",
            "image_url": {"url": "https://example.com/img.png"},
        }

    def test_image_only_message(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": "image/jpeg",
                                "data": "xyz",
                            },
                        },
                    ],
                }
            ],
        }
        result = anthropic_to_openai(body)
        content = result["messages"][0]["content"]
        assert isinstance(content, list)
        assert len(content) == 1
        assert content[0]["type"] == "image_url"

    def test_thinking_blocks_stripped(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [
                {
                    "role": "assistant",
                    "content": [
                        {
                            "type": "thinking",
                            "thinking": "Let me reason...",
                            "signature": "sig_abc",
                        },
                        {"type": "text", "text": "The answer is 42."},
                    ],
                },
            ],
        }
        result = anthropic_to_openai(body)
        assert result["messages"][0]["content"] == "The answer is 42."

    def test_redacted_thinking_blocks_stripped(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [
                {
                    "role": "assistant",
                    "content": [
                        {"type": "redacted_thinking", "data": "encrypted"},
                        {"type": "text", "text": "Here is my answer."},
                    ],
                },
            ],
        }
        result = anthropic_to_openai(body)
        assert result["messages"][0]["content"] == "Here is my answer."

    def test_thinking_only_message_produces_empty_content(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [
                {
                    "role": "assistant",
                    "content": [
                        {
                            "type": "thinking",
                            "thinking": "internal reasoning",
                            "signature": "sig",
                        },
                    ],
                },
            ],
        }
        result = anthropic_to_openai(body)
        assert result["messages"][0]["content"] == ""

    def test_unknown_block_types_stripped(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {"type": "text", "text": "Hello"},
                        {"type": "some_future_type", "data": "stuff"},
                    ],
                }
            ],
        }
        result = anthropic_to_openai(body)
        assert result["messages"][0]["content"] == "Hello"

    def test_converts_tools_to_openai_format(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "tools": [
                {
                    "name": "bash",
                    "description": "Run a bash command",
                    "input_schema": {
                        "type": "object",
                        "properties": {"command": {"type": "string"}},
                        "required": ["command"],
                    },
                }
            ],
            "messages": [{"role": "user", "content": "List files"}],
        }
        result = anthropic_to_openai(body)
        assert len(result["tools"]) == 1
        tool = result["tools"][0]
        assert tool["type"] == "function"
        assert tool["function"]["name"] == "bash"
        assert tool["function"]["description"] == "Run a bash command"
        assert tool["function"]["parameters"]["required"] == ["command"]

    def test_converts_tool_choice_auto(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "tool_choice": {"type": "auto"},
            "tools": [{"name": "bash", "description": "", "input_schema": {}}],
            "messages": [{"role": "user", "content": "Hi"}],
        }
        result = anthropic_to_openai(body)
        assert result["tool_choice"] == "auto"

    def test_converts_tool_choice_any(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "tool_choice": {"type": "any"},
            "tools": [{"name": "bash", "description": "", "input_schema": {}}],
            "messages": [{"role": "user", "content": "Hi"}],
        }
        result = anthropic_to_openai(body)
        assert result["tool_choice"] == "required"

    def test_converts_tool_choice_specific_tool(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "tool_choice": {"type": "tool", "name": "bash"},
            "tools": [{"name": "bash", "description": "", "input_schema": {}}],
            "messages": [{"role": "user", "content": "Hi"}],
        }
        result = anthropic_to_openai(body)
        assert result["tool_choice"] == {
            "type": "function",
            "function": {"name": "bash"},
        }

    def test_converts_assistant_tool_use_to_tool_calls(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "List files"},
                {
                    "role": "assistant",
                    "content": [
                        {"type": "text", "text": "I'll run ls."},
                        {
                            "type": "tool_use",
                            "id": "toolu_123",
                            "name": "bash",
                            "input": {"command": "ls"},
                        },
                    ],
                },
            ],
        }
        result = anthropic_to_openai(body)
        assistant_msg = result["messages"][1]
        assert assistant_msg["role"] == "assistant"
        assert assistant_msg["content"] == "I'll run ls."
        assert len(assistant_msg["tool_calls"]) == 1
        tc = assistant_msg["tool_calls"][0]
        assert tc["id"] == "toolu_123"
        assert tc["type"] == "function"
        assert tc["function"]["name"] == "bash"
        assert json.loads(tc["function"]["arguments"]) == {"command": "ls"}

    def test_converts_tool_result_to_tool_message(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "tool_result",
                            "tool_use_id": "toolu_123",
                            "content": "file1.txt\nfile2.txt",
                        }
                    ],
                },
            ],
        }
        result = anthropic_to_openai(body)
        tool_msg = result["messages"][0]
        assert tool_msg["role"] == "tool"
        assert tool_msg["tool_call_id"] == "toolu_123"
        assert tool_msg["content"] == "file1.txt\nfile2.txt"

    def test_full_tool_use_conversation(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "tools": [
                {
                    "name": "bash",
                    "description": "Run a command",
                    "input_schema": {
                        "type": "object",
                        "properties": {"command": {"type": "string"}},
                    },
                }
            ],
            "messages": [
                {"role": "user", "content": "List files"},
                {
                    "role": "assistant",
                    "content": [
                        {
                            "type": "tool_use",
                            "id": "toolu_1",
                            "name": "bash",
                            "input": {"command": "ls"},
                        },
                    ],
                },
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "tool_result",
                            "tool_use_id": "toolu_1",
                            "content": "README.md\napp/",
                        }
                    ],
                },
            ],
        }
        result = anthropic_to_openai(body)
        assert result["messages"][0] == {"role": "user", "content": "List files"}
        assert result["messages"][1]["role"] == "assistant"
        assert result["messages"][1]["content"] is None
        assert result["messages"][1]["tool_calls"][0]["function"]["name"] == "bash"
        assert result["messages"][2]["role"] == "tool"
        assert result["messages"][2]["tool_call_id"] == "toolu_1"
        assert result["messages"][2]["content"] == "README.md\napp/"
        assert len(result["tools"]) == 1

    def test_tool_result_with_content_blocks(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "tool_result",
                            "tool_use_id": "toolu_1",
                            "content": [
                                {"type": "text", "text": "line1"},
                                {"type": "text", "text": "line2"},
                            ],
                        }
                    ],
                },
            ],
        }
        result = anthropic_to_openai(body)
        assert result["messages"][0]["content"] == "line1\nline2"

    def test_tool_results_with_text_ordered_before_user_message(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "Run ls and pwd"},
                {
                    "role": "assistant",
                    "content": [
                        {
                            "type": "tool_use",
                            "id": "toolu_1",
                            "name": "bash",
                            "input": {"command": "ls"},
                        },
                        {
                            "type": "tool_use",
                            "id": "toolu_2",
                            "name": "bash",
                            "input": {"command": "pwd"},
                        },
                    ],
                },
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "tool_result",
                            "tool_use_id": "toolu_1",
                            "content": "file1.txt",
                        },
                        {
                            "type": "tool_result",
                            "tool_use_id": "toolu_2",
                            "content": "/home/user",
                        },
                        {"type": "text", "text": "Now explain the results"},
                    ],
                },
            ],
        }
        result = anthropic_to_openai(body)
        assert result["messages"][0] == {"role": "user", "content": "Run ls and pwd"}
        assert result["messages"][1]["role"] == "assistant"
        assert len(result["messages"][1]["tool_calls"]) == 2
        assert result["messages"][2]["role"] == "tool"
        assert result["messages"][2]["tool_call_id"] == "toolu_1"
        assert result["messages"][3]["role"] == "tool"
        assert result["messages"][3]["tool_call_id"] == "toolu_2"
        assert result["messages"][4] == {
            "role": "user",
            "content": "Now explain the results",
        }

    def test_multiple_tool_results_without_text(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "tool_result",
                            "tool_use_id": "toolu_a",
                            "content": "ok",
                        },
                        {
                            "type": "tool_result",
                            "tool_use_id": "toolu_b",
                            "content": "done",
                        },
                    ],
                },
            ],
        }
        result = anthropic_to_openai(body)
        assert len(result["messages"]) == 2
        assert result["messages"][0]["role"] == "tool"
        assert result["messages"][0]["tool_call_id"] == "toolu_a"
        assert result["messages"][1]["role"] == "tool"
        assert result["messages"][1]["tool_call_id"] == "toolu_b"

    def test_no_tools_key_when_absent(self):
        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Hi"}],
        }
        result = anthropic_to_openai(body)
        assert "tools" not in result
        assert "tool_choice" not in result


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

    def test_tool_calls_converted_to_tool_use_blocks(self):
        tc_func = MagicMock()
        tc_func.name = "bash"
        tc_func.arguments = '{"command": "ls"}'
        tc = MagicMock()
        tc.id = "call_123"
        tc.function = tc_func

        choice = MagicMock()
        choice.message.content = None
        choice.message.tool_calls = [tc]
        choice.finish_reason = "tool_calls"

        response = MagicMock()
        response.choices = [choice]
        response.usage = MagicMock(prompt_tokens=10, completion_tokens=5)

        result = openai_response_to_anthropic(response, "test/model")
        assert result["stop_reason"] == "tool_use"
        assert len(result["content"]) == 1
        block = result["content"][0]
        assert block["type"] == "tool_use"
        assert block["id"] == "call_123"
        assert block["name"] == "bash"
        assert block["input"] == {"command": "ls"}

    def test_text_and_tool_calls_together(self):
        tc_func = MagicMock()
        tc_func.name = "bash"
        tc_func.arguments = '{"command": "ls"}'
        tc = MagicMock()
        tc.id = "call_456"
        tc.function = tc_func

        choice = MagicMock()
        choice.message.content = "I'll run that."
        choice.message.tool_calls = [tc]
        choice.finish_reason = "tool_calls"

        response = MagicMock()
        response.choices = [choice]
        response.usage = MagicMock(prompt_tokens=10, completion_tokens=5)

        result = openai_response_to_anthropic(response, "test/model")
        assert len(result["content"]) == 2
        assert result["content"][0] == {"type": "text", "text": "I'll run that."}
        assert result["content"][1]["type"] == "tool_use"
        assert result["content"][1]["name"] == "bash"

    def test_no_tool_calls_returns_text_only(self):
        choice = MagicMock()
        choice.message.content = "Hello!"
        choice.message.tool_calls = None
        choice.finish_reason = "stop"

        response = MagicMock()
        response.choices = [choice]
        response.usage = MagicMock(prompt_tokens=10, completion_tokens=5)

        result = openai_response_to_anthropic(response, "test/model")
        assert result["stop_reason"] == "end_turn"
        assert len(result["content"]) == 1
        assert result["content"][0] == {"type": "text", "text": "Hello!"}

from __future__ import annotations

from app.proxy.message_sanitizer import sanitize_messages, sanitize_tools


class TestSanitizeMessages:
    def test_plain_string_messages_unchanged(self):
        messages = [
            {"role": "system", "content": "You are helpful"},
            {"role": "user", "content": "Hello"},
            {"role": "assistant", "content": "Hi"},
        ]
        assert sanitize_messages(messages) == messages

    def test_openai_content_list_unchanged(self):
        messages = [
            {
                "role": "user",
                "content": [{"type": "text", "text": "Describe this image"}],
            }
        ]
        assert sanitize_messages(messages) == messages

    def test_tool_result_converted_to_tool_role(self):
        messages = [
            {
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": "call_123",
                        "content": "file contents here",
                    }
                ],
            }
        ]
        result = sanitize_messages(messages)
        assert len(result) == 1
        assert result[0]["role"] == "tool"
        assert result[0]["tool_call_id"] == "call_123"
        assert result[0]["content"] == "file contents here"

    def test_tool_use_converted_to_tool_calls(self):
        messages = [
            {
                "role": "assistant",
                "content": [
                    {
                        "type": "tool_use",
                        "id": "call_456",
                        "name": "read_file",
                        "input": {"path": "/tmp/test.py"},
                    }
                ],
            }
        ]
        result = sanitize_messages(messages)
        assert len(result) == 1
        assert result[0]["role"] == "assistant"
        assert result[0]["content"] is None
        assert len(result[0]["tool_calls"]) == 1
        tc = result[0]["tool_calls"][0]
        assert tc["id"] == "call_456"
        assert tc["type"] == "function"
        assert tc["function"]["name"] == "read_file"
        assert '"path": "/tmp/test.py"' in tc["function"]["arguments"]

    def test_mixed_text_and_tool_use(self):
        messages = [
            {
                "role": "assistant",
                "content": [
                    {"type": "text", "text": "Let me read that file."},
                    {
                        "type": "tool_use",
                        "id": "call_789",
                        "name": "read_file",
                        "input": {"path": "a.py"},
                    },
                ],
            }
        ]
        result = sanitize_messages(messages)
        assert len(result) == 1
        assert result[0]["role"] == "assistant"
        assert result[0]["content"] is None
        assert len(result[0]["tool_calls"]) == 1

    def test_mixed_text_and_tool_result(self):
        messages = [
            {
                "role": "user",
                "content": [
                    {"type": "text", "text": "Here is the result"},
                    {
                        "type": "tool_result",
                        "tool_use_id": "call_abc",
                        "content": "output data",
                    },
                ],
            }
        ]
        result = sanitize_messages(messages)
        assert len(result) == 2
        assert result[0]["role"] == "user"
        assert result[0]["content"] == "Here is the result"
        assert result[1]["role"] == "tool"
        assert result[1]["tool_call_id"] == "call_abc"

    def test_tool_result_with_nested_content_list(self):
        messages = [
            {
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": "call_nested",
                        "content": [
                            {"type": "text", "text": "line 1"},
                            {"type": "text", "text": "line 2"},
                        ],
                    }
                ],
            }
        ]
        result = sanitize_messages(messages)
        assert result[0]["content"] == "line 1\nline 2"

    def test_multiple_tool_results_in_one_message(self):
        messages = [
            {
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": "call_1",
                        "content": "result 1",
                    },
                    {
                        "type": "tool_result",
                        "tool_use_id": "call_2",
                        "content": "result 2",
                    },
                ],
            }
        ]
        result = sanitize_messages(messages)
        assert len(result) == 2
        assert result[0]["role"] == "tool"
        assert result[0]["tool_call_id"] == "call_1"
        assert result[1]["role"] == "tool"
        assert result[1]["tool_call_id"] == "call_2"

    def test_empty_messages_list(self):
        assert sanitize_messages([]) == []

    def test_preserves_non_content_fields(self):
        messages = [{"role": "user", "content": "hello", "name": "user1"}]
        result = sanitize_messages(messages)
        assert result[0]["name"] == "user1"

    def test_multiple_tool_uses_in_one_message(self):
        messages = [
            {
                "role": "assistant",
                "content": [
                    {
                        "type": "tool_use",
                        "id": "call_a",
                        "name": "read_file",
                        "input": {"path": "a.py"},
                    },
                    {
                        "type": "tool_use",
                        "id": "call_b",
                        "name": "read_file",
                        "input": {"path": "b.py"},
                    },
                ],
            }
        ]
        result = sanitize_messages(messages)
        assert len(result) == 1
        assert len(result[0]["tool_calls"]) == 2


class TestSanitizeTools:
    def test_openai_format_tools_unchanged(self):
        tools = [
            {
                "type": "function",
                "function": {
                    "name": "read_file",
                    "description": "Read a file",
                    "parameters": {
                        "type": "object",
                        "properties": {"path": {"type": "string"}},
                    },
                },
            }
        ]
        result = sanitize_tools(tools)
        assert result == tools

    def test_anthropic_format_converted_to_openai(self):
        tools = [
            {
                "name": "read_file",
                "description": "Read a file",
                "input_schema": {
                    "type": "object",
                    "properties": {"path": {"type": "string"}},
                },
            }
        ]
        result = sanitize_tools(tools)
        assert len(result) == 1
        assert result[0]["type"] == "function"
        assert result[0]["function"]["name"] == "read_file"
        assert result[0]["function"]["description"] == "Read a file"
        assert result[0]["function"]["parameters"] == {
            "type": "object",
            "properties": {"path": {"type": "string"}},
        }

    def test_mixed_formats(self):
        tools = [
            {
                "type": "function",
                "function": {
                    "name": "tool_a",
                    "description": "Already OpenAI",
                    "parameters": {},
                },
            },
            {
                "name": "tool_b",
                "description": "Anthropic format",
                "input_schema": {"type": "object", "properties": {}},
            },
        ]
        result = sanitize_tools(tools)
        assert result[0]["function"]["name"] == "tool_a"
        assert result[1]["type"] == "function"
        assert result[1]["function"]["name"] == "tool_b"

    def test_empty_tools_list(self):
        assert sanitize_tools([]) == []

    def test_anthropic_tool_missing_input_schema(self):
        tools = [{"name": "simple_tool", "description": "No schema"}]
        result = sanitize_tools(tools)
        assert result[0]["function"]["name"] == "simple_tool"
        assert result[0]["function"]["parameters"] == {}

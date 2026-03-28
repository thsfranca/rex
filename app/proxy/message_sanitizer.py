from __future__ import annotations

import json

from app.proxy.anthropic import _convert_image_block

_ANTHROPIC_BLOCK_TYPES = frozenset(
    {"tool_result", "tool_use", "thinking", "redacted_thinking", "image"}
)


def sanitize_tools(tools: list[dict]) -> list[dict]:
    return [_normalize_tool(t) for t in tools]


def _normalize_tool(tool: dict) -> dict:
    if "function" in tool:
        return tool
    return {
        "type": "function",
        "function": {
            "name": tool.get("name", ""),
            "description": tool.get("description", ""),
            "parameters": tool.get("input_schema", {}),
        },
    }


def sanitize_messages(messages: list[dict]) -> list[dict]:
    result: list[dict] = []
    for msg in messages:
        converted = _convert_message(msg)
        result.extend(converted)
    return result


def _convert_message(msg: dict) -> list[dict]:
    content = msg.get("content")
    if not isinstance(content, list):
        return [msg]

    has_anthropic_blocks = any(
        isinstance(block, dict) and block.get("type") in _ANTHROPIC_BLOCK_TYPES for block in content
    )
    if not has_anthropic_blocks:
        return [msg]

    output: list[dict] = []
    text_parts: list[str] = []
    image_parts: list[dict] = []
    tool_calls: list[dict] = []

    for block in content:
        if not isinstance(block, dict):
            continue

        block_type = block.get("type")

        if block_type == "text":
            text = block.get("text", "")
            if text:
                text_parts.append(text)

        elif block_type == "image":
            converted = _convert_image_block(block)
            if converted:
                image_parts.append(converted)

        elif block_type == "tool_use":
            tool_calls.append(
                {
                    "id": block.get("id", ""),
                    "type": "function",
                    "function": {
                        "name": block.get("name", ""),
                        "arguments": json.dumps(block.get("input", {})),
                    },
                }
            )

        elif block_type == "tool_result":
            if text_parts or image_parts:
                _flush_content(output, msg["role"], text_parts, image_parts)
                text_parts = []
                image_parts = []
            if tool_calls:
                assistant_msg: dict = {"role": "assistant", "content": None}
                assistant_msg["tool_calls"] = tool_calls
                output.append(assistant_msg)
                tool_calls = []

            result_content = block.get("content", "")
            if isinstance(result_content, list):
                result_content = "\n".join(
                    b.get("text", "") for b in result_content if isinstance(b, dict)
                )
            output.append(
                {
                    "role": "tool",
                    "tool_call_id": block.get("tool_use_id", ""),
                    "content": str(result_content),
                }
            )

    if tool_calls:
        assistant_msg = {"role": msg["role"], "content": None}
        assistant_msg["tool_calls"] = tool_calls
        output.append(assistant_msg)
    elif text_parts or image_parts:
        _flush_content(output, msg["role"], text_parts, image_parts)

    return output if output else [{"role": msg["role"], "content": ""}]


def _flush_content(
    output: list[dict], role: str, text_parts: list[str], image_parts: list[dict]
) -> None:
    if image_parts:
        content_list: list[dict] = []
        if text_parts:
            content_list.append({"type": "text", "text": "\n".join(text_parts)})
        content_list.extend(image_parts)
        output.append({"role": role, "content": content_list})
    elif text_parts:
        output.append({"role": role, "content": "\n".join(text_parts)})

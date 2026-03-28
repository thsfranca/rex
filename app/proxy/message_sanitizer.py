from __future__ import annotations

import json


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
        isinstance(block, dict) and block.get("type") in ("tool_result", "tool_use")
        for block in content
    )
    if not has_anthropic_blocks:
        return [msg]

    output: list[dict] = []
    text_parts: list[str] = []
    tool_calls: list[dict] = []

    for block in content:
        if not isinstance(block, dict):
            continue

        block_type = block.get("type")

        if block_type == "text":
            text = block.get("text", "")
            if text:
                text_parts.append(text)

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
            if text_parts:
                output.append({"role": msg["role"], "content": "\n".join(text_parts)})
                text_parts = []
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
    elif text_parts:
        output.append({"role": msg["role"], "content": "\n".join(text_parts)})

    return output if output else [{"role": msg["role"], "content": ""}]

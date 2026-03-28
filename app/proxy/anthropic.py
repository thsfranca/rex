from __future__ import annotations

import json
import logging
import time
import uuid
from collections.abc import AsyncIterator

from fastapi import Request

logger = logging.getLogger(__name__)


def extract_anthropic_api_key(request: Request) -> str | None:
    return request.headers.get("x-api-key")


def _convert_anthropic_tool(tool: dict) -> dict:
    return {
        "type": "function",
        "function": {
            "name": tool.get("name", ""),
            "description": tool.get("description", ""),
            "parameters": tool.get("input_schema", {}),
        },
    }


def _convert_anthropic_tool_choice(tool_choice: dict) -> str | dict:
    choice_type = tool_choice.get("type", "auto")
    if choice_type == "auto":
        return "auto"
    if choice_type == "any":
        return "required"
    if choice_type == "tool":
        return {"type": "function", "function": {"name": tool_choice.get("name", "")}}
    return "auto"


def _convert_image_block(block: dict) -> dict | None:
    source = block.get("source", {})
    source_type = source.get("type")
    if source_type == "base64":
        media_type = source.get("media_type", "image/jpeg")
        data = source.get("data", "")
        return {
            "type": "image_url",
            "image_url": {"url": f"data:{media_type};base64,{data}"},
        }
    if source_type == "url":
        return {
            "type": "image_url",
            "image_url": {"url": source.get("url", "")},
        }
    return None


def _convert_anthropic_message(role: str, content: list) -> list[dict]:
    output: list[dict] = []
    text_parts: list[str] = []
    image_parts: list[dict] = []
    tool_calls: list[dict] = []

    def flush_text_and_images() -> None:
        if not text_parts and not image_parts:
            return
        if image_parts:
            content_list: list[dict] = []
            if text_parts:
                content_list.append({"type": "text", "text": "\n".join(text_parts)})
            content_list.extend(image_parts)
            output.append({"role": role, "content": content_list})
        else:
            output.append({"role": role, "content": "\n".join(text_parts)})
        text_parts.clear()
        image_parts.clear()

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
            flush_text_and_images()
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
        assistant_msg: dict = {"role": role}
        assistant_msg["content"] = "\n".join(text_parts) if text_parts else None
        assistant_msg["tool_calls"] = tool_calls
        output.append(assistant_msg)
    elif text_parts or image_parts:
        flush_text_and_images()

    if not output:
        output.append({"role": role, "content": ""})

    return output


def anthropic_to_openai(body: dict) -> dict:
    openai_body: dict = {}

    messages: list[dict] = []
    system = body.get("system")
    if system:
        if isinstance(system, str):
            messages.append({"role": "system", "content": system})
        elif isinstance(system, list):
            text_parts = [
                block.get("text", "")
                for block in system
                if isinstance(block, dict) and block.get("type") == "text"
            ]
            if text_parts:
                messages.append({"role": "system", "content": "\n".join(text_parts)})

    for msg in body.get("messages", []):
        content = msg.get("content")
        if isinstance(content, str):
            messages.append({"role": msg["role"], "content": content})
        elif isinstance(content, list):
            messages.extend(_convert_anthropic_message(msg["role"], content))
        else:
            messages.append({"role": msg["role"], "content": ""})

    openai_body["messages"] = messages

    tools = body.get("tools")
    if tools:
        openai_body["tools"] = [_convert_anthropic_tool(t) for t in tools]

    tool_choice = body.get("tool_choice")
    if tool_choice:
        openai_body["tool_choice"] = _convert_anthropic_tool_choice(tool_choice)

    if "max_tokens" in body:
        openai_body["max_tokens"] = body["max_tokens"]
    if "temperature" in body:
        openai_body["temperature"] = body["temperature"]
    if "top_p" in body:
        openai_body["top_p"] = body["top_p"]
    if "stop_sequences" in body:
        openai_body["stop"] = body["stop_sequences"]

    return openai_body


_STOP_REASON_MAP = {
    "stop": "end_turn",
    "length": "max_tokens",
    "tool_calls": "tool_use",
}


def openai_response_to_anthropic(
    response, model_name: str, request_model: str | None = None
) -> dict:
    choice = response.choices[0] if response.choices else None

    content_blocks: list[dict] = []

    if choice and choice.message:
        if choice.message.content:
            content_blocks.append({"type": "text", "text": choice.message.content})

        if hasattr(choice.message, "tool_calls") and choice.message.tool_calls:
            for tc in choice.message.tool_calls:
                tool_input: dict = {}
                if hasattr(tc, "function") and tc.function:
                    try:
                        tool_input = json.loads(tc.function.arguments)
                    except (json.JSONDecodeError, TypeError):
                        tool_input = {}

                content_blocks.append(
                    {
                        "type": "tool_use",
                        "id": tc.id,
                        "name": tc.function.name if hasattr(tc, "function") and tc.function else "",
                        "input": tool_input,
                    }
                )

    if not content_blocks:
        content_blocks.append({"type": "text", "text": ""})

    finish_reason = choice.finish_reason if choice else None
    stop_reason = _STOP_REASON_MAP.get(finish_reason, "end_turn")

    input_tokens = 0
    output_tokens = 0
    if response.usage:
        input_tokens = getattr(response.usage, "prompt_tokens", 0) or 0
        output_tokens = getattr(response.usage, "completion_tokens", 0) or 0

    return {
        "id": f"msg_{uuid.uuid4().hex[:24]}",
        "type": "message",
        "role": "assistant",
        "content": content_blocks,
        "model": request_model or model_name,
        "stop_reason": stop_reason,
        "stop_sequence": None,
        "usage": {
            "input_tokens": input_tokens,
            "output_tokens": output_tokens,
        },
    }


def _sse_event(event_type: str, data: dict) -> str:
    return f"event: {event_type}\ndata: {json.dumps(data)}\n\n"


async def stream_anthropic_response(
    response,
    model_name: str,
    request_model: str | None = None,
    timeout: float = 600,
) -> AsyncIterator[str]:
    msg_id = f"msg_{uuid.uuid4().hex[:24]}"
    model = request_model or model_name

    yield _sse_event(
        "message_start",
        {
            "type": "message_start",
            "message": {
                "id": msg_id,
                "type": "message",
                "role": "assistant",
                "content": [],
                "model": model,
                "stop_reason": None,
                "stop_sequence": None,
                "usage": {"input_tokens": 0, "output_tokens": 0},
            },
        },
    )

    yield _sse_event(
        "content_block_start",
        {
            "type": "content_block_start",
            "index": 0,
            "content_block": {"type": "text", "text": ""},
        },
    )

    yield _sse_event("ping", {"type": "ping"})

    output_tokens = 0
    finish_reason = None
    text_block_closed = False
    tool_block_indices: dict[int, int] = {}
    next_block_index = 1
    deadline = time.perf_counter() + timeout

    async for chunk in response:
        if time.perf_counter() > deadline:
            logger.warning("Anthropic stream wall-clock limit reached (%.1fs)", timeout)
            break
        delta_content = None
        delta_tool_calls = None
        if chunk.choices:
            delta = chunk.choices[0].delta
            if hasattr(delta, "content") and delta.content:
                delta_content = delta.content
            if hasattr(delta, "tool_calls") and delta.tool_calls:
                delta_tool_calls = delta.tool_calls
            if chunk.choices[0].finish_reason:
                finish_reason = chunk.choices[0].finish_reason

        if hasattr(chunk, "usage") and chunk.usage:
            if hasattr(chunk.usage, "completion_tokens") and chunk.usage.completion_tokens:
                output_tokens = chunk.usage.completion_tokens

        if delta_content:
            output_tokens += 1
            yield _sse_event(
                "content_block_delta",
                {
                    "type": "content_block_delta",
                    "index": 0,
                    "delta": {"type": "text_delta", "text": delta_content},
                },
            )

        if delta_tool_calls:
            if not text_block_closed:
                yield _sse_event(
                    "content_block_stop",
                    {"type": "content_block_stop", "index": 0},
                )
                text_block_closed = True

            for tc in delta_tool_calls:
                tc_idx = tc.index if hasattr(tc, "index") else 0

                if tc_idx not in tool_block_indices:
                    block_idx = next_block_index
                    tool_block_indices[tc_idx] = block_idx
                    next_block_index += 1

                    tc_id = getattr(tc, "id", None) or ""
                    tc_name = ""
                    if hasattr(tc, "function") and tc.function:
                        tc_name = getattr(tc.function, "name", None) or ""

                    yield _sse_event(
                        "content_block_start",
                        {
                            "type": "content_block_start",
                            "index": block_idx,
                            "content_block": {
                                "type": "tool_use",
                                "id": tc_id,
                                "name": tc_name,
                                "input": {},
                            },
                        },
                    )

                block_idx = tool_block_indices[tc_idx]
                if hasattr(tc, "function") and tc.function:
                    args = getattr(tc.function, "arguments", None)
                    if args:
                        yield _sse_event(
                            "content_block_delta",
                            {
                                "type": "content_block_delta",
                                "index": block_idx,
                                "delta": {
                                    "type": "input_json_delta",
                                    "partial_json": args,
                                },
                            },
                        )

    if not text_block_closed:
        yield _sse_event(
            "content_block_stop",
            {"type": "content_block_stop", "index": 0},
        )

    for tc_idx in sorted(tool_block_indices.keys()):
        block_idx = tool_block_indices[tc_idx]
        yield _sse_event(
            "content_block_stop",
            {"type": "content_block_stop", "index": block_idx},
        )

    stop_reason = _STOP_REASON_MAP.get(finish_reason, "end_turn")
    yield _sse_event(
        "message_delta",
        {
            "type": "message_delta",
            "delta": {"stop_reason": stop_reason, "stop_sequence": None},
            "usage": {"output_tokens": output_tokens},
        },
    )

    yield _sse_event("message_stop", {"type": "message_stop"})

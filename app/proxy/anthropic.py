from __future__ import annotations

import json
import uuid
from collections.abc import AsyncIterator

from fastapi import Request


def extract_anthropic_api_key(request: Request) -> str | None:
    return request.headers.get("x-api-key")


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
        openai_msg: dict = {"role": msg["role"]}
        content = msg.get("content")
        if isinstance(content, str):
            openai_msg["content"] = content
        elif isinstance(content, list):
            text_parts = [
                block.get("text", "")
                for block in content
                if isinstance(block, dict) and block.get("type") == "text"
            ]
            openai_msg["content"] = "\n".join(text_parts) if text_parts else ""
        else:
            openai_msg["content"] = ""
        messages.append(openai_msg)

    openai_body["messages"] = messages

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
}


def openai_response_to_anthropic(
    response, model_name: str, request_model: str | None = None
) -> dict:
    choice = response.choices[0] if response.choices else None
    text = ""
    if choice and choice.message and choice.message.content:
        text = choice.message.content

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
        "content": [{"type": "text", "text": text}],
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

    async for chunk in response:
        delta_content = None
        if chunk.choices:
            delta = chunk.choices[0].delta
            if hasattr(delta, "content") and delta.content:
                delta_content = delta.content
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

    yield _sse_event(
        "content_block_stop",
        {"type": "content_block_stop", "index": 0},
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

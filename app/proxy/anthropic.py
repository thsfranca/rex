from __future__ import annotations

import uuid

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

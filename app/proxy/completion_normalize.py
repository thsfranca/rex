from __future__ import annotations

import json

_JSON_WRAPPER_KEYS = frozenset({"response", "text", "message", "answer", "content"})


def is_ollama_litellm_model(model_name: str) -> bool:
    return model_name.startswith(("ollama/", "ollama_chat/"))


def unwrap_llm_json_text_wrapper(content: str | None) -> str | None:
    if content is None:
        return None
    s = content.strip()
    if not s or s[0] != "{":
        return content
    try:
        obj = json.loads(s)
    except json.JSONDecodeError:
        return content
    if not isinstance(obj, dict) or len(obj) != 1:
        return content
    key, val = next(iter(obj.items()))
    if key in _JSON_WRAPPER_KEYS and isinstance(val, str):
        return val
    return content


def apply_ollama_completion_text_unwrap(response, model_name: str) -> None:
    if not is_ollama_litellm_model(model_name):
        return
    if not getattr(response, "choices", None):
        return
    choice = response.choices[0]
    if not choice:
        return
    msg = getattr(choice, "message", None)
    if msg is not None:
        if getattr(msg, "tool_calls", None):
            return
        unwrapped = unwrap_llm_json_text_wrapper(msg.content)
        if unwrapped is not None:
            msg.content = unwrapped
        return
    if hasattr(choice, "text"):
        unwrapped = unwrap_llm_json_text_wrapper(choice.text)
        if unwrapped is not None:
            choice.text = unwrapped

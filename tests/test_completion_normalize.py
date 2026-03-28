from litellm import ModelResponse, TextCompletionResponse
from litellm.types.utils import Choices, Message, TextChoices, Usage

from app.config import Model
from app.proxy.completion_normalize import (
    apply_ollama_completion_text_unwrap,
    is_ollama_litellm_model,
    unwrap_llm_json_text_wrapper,
)
from app.proxy.handler import _build_litellm_params


def test_unwrap_llm_json_text_wrapper_response_key():
    assert unwrap_llm_json_text_wrapper('{"response": "No."}') == "No."


def test_unwrap_llm_json_text_wrapper_preserves_multi_key_json():
    raw = '{"a": 1, "b": 2}'
    assert unwrap_llm_json_text_wrapper(raw) == raw


def test_unwrap_llm_json_text_wrapper_preserves_plain_text():
    assert unwrap_llm_json_text_wrapper("plain") == "plain"


def test_unwrap_llm_json_text_wrapper_none():
    assert unwrap_llm_json_text_wrapper(None) is None


def test_is_ollama_litellm_model():
    assert is_ollama_litellm_model("ollama/mistral:latest")
    assert is_ollama_litellm_model("ollama_chat/mistral")
    assert not is_ollama_litellm_model("openai/gpt-4o")


def test_apply_ollama_completion_text_unwrap_skips_non_ollama():
    c = Choices(
        finish_reason="stop",
        index=0,
        message=Message(content='{"response": "x"}', role="assistant"),
    )
    r = ModelResponse(choices=[c], model="openai/gpt-4o", usage=Usage())
    apply_ollama_completion_text_unwrap(r, "openai/gpt-4o")
    assert r.choices[0].message.content == '{"response": "x"}'


def test_apply_ollama_completion_text_unwrap_chat_message():
    c = Choices(
        finish_reason="stop",
        index=0,
        message=Message(content='{"response": "hello"}', role="assistant"),
    )
    r = ModelResponse(choices=[c], model="ollama/mistral:latest", usage=Usage())
    apply_ollama_completion_text_unwrap(r, "ollama/mistral:latest")
    assert r.choices[0].message.content == "hello"


def test_apply_ollama_completion_text_unwrap_legacy_text_choice():
    c = TextChoices(finish_reason="stop", index=0, text='{"text": "hi"}')
    r = TextCompletionResponse(choices=[c], model="ollama/m", usage=Usage())
    apply_ollama_completion_text_unwrap(r, "ollama/mistral:latest")
    assert r.choices[0].text == "hi"


def test_build_litellm_params_drops_response_format_for_ollama():
    params = _build_litellm_params(
        {"messages": [], "response_format": {"type": "json_object"}},
        Model(name="ollama/mistral:latest"),
    )
    assert "response_format" not in params


def test_build_litellm_params_keeps_response_format_for_openai():
    rf = {"type": "json_object"}
    params = _build_litellm_params(
        {"messages": [], "response_format": rf},
        Model(name="openai/gpt-4o"),
    )
    assert params.get("response_format") == rf

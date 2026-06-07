"""BrokerChatModel and streaming buffer tests."""

from rex_agent.broker import BrokerToolCall, InferenceResult, legacy_inference_result
from rex_agent.broker_chat_model import route_inference_result, stream_visible_text


def test_stream_visible_text_hides_tool_json() -> None:
    raw = '{"type":"tool","tool":"fs.read","args":{"path":"a"}}'
    visible = "".join(stream_visible_text(raw))
    assert "fs.read" not in visible


def test_stream_visible_text_passes_final_answer() -> None:
    visible = "".join(stream_visible_text("hello world"))
    assert "hello" in visible


def test_route_inference_result_native_tool_calls() -> None:
    result = InferenceResult(
        ok=True,
        tool_calls=[BrokerToolCall(tool="fs.read", args={"path": "a.md"})],
    )
    ai, parsed = route_inference_result(result, "plan")
    assert parsed is None
    assert ai.tool_calls
    assert ai.tool_calls[0]["name"] == "fs.read"


def test_route_inference_result_interim_final() -> None:
    result = legacy_inference_result(True, "plain answer")
    ai, parsed = route_inference_result(result, "ask")
    assert parsed is not None
    assert parsed.kind == "final"
    assert parsed.answer == "plain answer"
    assert ai.content == "plain answer"

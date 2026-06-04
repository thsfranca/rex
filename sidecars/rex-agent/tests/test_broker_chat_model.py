"""BrokerChatModel and streaming buffer tests."""

from rex_agent.broker_chat_model import stream_visible_text


def test_stream_visible_text_hides_tool_json() -> None:
    raw = '{"type":"tool","tool":"fs.read","args":{"path":"a"}}'
    visible = "".join(stream_visible_text(raw))
    assert "fs.read" not in visible


def test_stream_visible_text_passes_final_answer() -> None:
    visible = "".join(stream_visible_text("hello world"))
    assert "hello" in visible

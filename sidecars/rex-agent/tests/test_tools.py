"""Tool protocol parsing tests."""

from rex_agent.tools import (
    TOOL_LIST,
    TOOL_READ,
    parse_model_output,
    tools_for_mode,
)


def test_plan_mode_allows_read_and_list() -> None:
    allowed = tools_for_mode("plan")
    assert TOOL_READ in allowed
    assert TOOL_LIST in allowed


def test_ask_mode_parses_plain_text_as_final() -> None:
    parsed = parse_model_output("hello stub", "ask")
    assert parsed.kind == "final"
    assert parsed.answer == "hello stub"


def test_tool_json_parsed() -> None:
    raw = '{"type":"tool","tool":"fs.read","args":{"path":"README.md"}}'
    parsed = parse_model_output(raw, "plan")
    assert parsed.kind == "tool"
    assert parsed.tool_call is not None
    assert parsed.tool_call.tool == "fs.read"


def test_plan_mode_rejects_write_tool() -> None:
    raw = '{"type":"tool","tool":"fs.write","args":{"path":"a.txt","content":"x"}}'
    parsed = parse_model_output(raw, "plan")
    assert parsed.kind == "error"

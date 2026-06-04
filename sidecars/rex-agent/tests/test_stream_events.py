"""Tests for structured stream event helpers."""

from __future__ import annotations

from rex_agent.graph.stream_queue import append_step, append_tool
from rex_agent.stream_events import cap_detail, tool_detail_from_call
from rex_agent.tools import ToolCall


def test_cap_detail_truncates_long_text() -> None:
    long = "x" * 300
    assert len(cap_detail(long)) == 240
    assert cap_detail(long).endswith("...")


def test_tool_detail_from_call_prefers_path() -> None:
    call = ToolCall(tool="fs.read", args={"path": "src/main.rs"})
    assert tool_detail_from_call(call) == "src/main.rs"


def test_append_tool_and_step_build_event_list() -> None:
    events = append_step([], phase="running", summary="Routing to viewer for fs.read")
    events = append_tool(events, name="fs.read", phase="running", detail="src/main.rs")
    events = append_tool(events, name="fs.read", phase="completed", detail="ok")
    assert len(events) == 3

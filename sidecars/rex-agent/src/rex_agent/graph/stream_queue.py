"""In-turn ordered stream events (tool/step/text) for RunTurn streaming."""

from __future__ import annotations

from typing import Annotated

from rex_agent.graph.stream_sink import active_sink
from rex_agent.stream_events import (
    ActivityStreamEvent,
    PlanStreamEvent,
    StepStreamEvent,
    StreamEvent,
    TextStreamEvent,
    ToolStreamEvent,
)


def merge_stream_events(
    left: list[StreamEvent], right: list[StreamEvent]
) -> list[StreamEvent]:
    return left + right


StreamEventList = Annotated[list[StreamEvent], merge_stream_events]


def append_text(events: list[StreamEvent], text: str) -> list[StreamEvent]:
    if not text:
        return events
    sink = active_sink()
    if sink is not None:
        sink.emit_text(text)
    return events + [TextStreamEvent(text=text)]


def append_tool(
    events: list[StreamEvent],
    *,
    name: str,
    phase: str,
    detail: str = "",
    tool_call_id: str = "",
) -> list[StreamEvent]:
    sink = active_sink()
    if sink is not None:
        sink.emit_tool(
            name=name,
            phase=phase,
            detail=detail,
            tool_call_id=tool_call_id,
        )
    return events + [
        ToolStreamEvent(
            name=name,
            phase=phase,
            detail=detail,
            tool_call_id=tool_call_id,
        )
    ]


def append_step(
    events: list[StreamEvent],
    *,
    phase: str,
    summary: str,
) -> list[StreamEvent]:
    sink = active_sink()
    if sink is not None:
        sink.emit_step(phase=phase, summary=summary)
    return events + [StepStreamEvent(phase=phase, summary=summary)]


def append_plan(
    events: list[StreamEvent],
    *,
    phase: str,
    title: str,
    detail: str = "",
) -> list[StreamEvent]:
    sink = active_sink()
    if sink is not None:
        sink.emit_plan(phase=phase, title=title, detail=detail)
    return events + [PlanStreamEvent(phase=phase, title=title, detail=detail)]


def append_activity(
    events: list[StreamEvent],
    *,
    phase: str,
    summary: str,
    detail: str = "",
) -> list[StreamEvent]:
    sink = active_sink()
    if sink is not None:
        sink.emit_activity(phase=phase, summary=summary, detail=detail)
    return events + [
        ActivityStreamEvent(phase=phase, summary=summary, detail=detail)
    ]

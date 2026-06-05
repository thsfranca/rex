"""In-turn ordered stream events (tool/step/text) for RunTurn streaming."""

from __future__ import annotations

from typing import Annotated

from rex_agent.stream_events import (
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
    return events + [TextStreamEvent(text=text)]


def append_tool(
    events: list[StreamEvent],
    *,
    name: str,
    phase: str,
    detail: str = "",
) -> list[StreamEvent]:
    return events + [ToolStreamEvent(name=name, phase=phase, detail=detail)]


def append_step(
    events: list[StreamEvent],
    *,
    phase: str,
    summary: str,
) -> list[StreamEvent]:
    return events + [StepStreamEvent(phase=phase, summary=summary)]


def append_plan(
    events: list[StreamEvent],
    *,
    phase: str,
    title: str,
    detail: str = "",
) -> list[StreamEvent]:
    return events + [PlanStreamEvent(phase=phase, title=title, detail=detail)]

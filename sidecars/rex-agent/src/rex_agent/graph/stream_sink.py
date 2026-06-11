"""Live per-turn event queue for incremental RunTurn streaming (ADR 0030)."""

from __future__ import annotations

import contextvars
import time
from collections import deque
from dataclasses import dataclass, field
from typing import Iterator

from rex_agent.stream_events import (
    ActivityStreamEvent,
    PlanStreamEvent,
    StepStreamEvent,
    StreamEvent,
    TextStreamEvent,
    ToolStreamEvent,
)

_active_sink: contextvars.ContextVar[LiveStreamSink | None] = contextvars.ContextVar(
    "rex_agent_stream_sink", default=None
)


@dataclass
class LiveStreamSink:
    """Per-turn queue drained by RunTurn while the graph streams."""

    turn_id: str = ""
    _queue: deque[StreamEvent] = field(default_factory=deque, init=False, repr=False)
    _sequence: int = field(default=0, init=False)
    _tool_starts: dict[str, float] = field(default_factory=dict, init=False, repr=False)

    def _next_sequence(self) -> int:
        self._sequence += 1
        return self._sequence

    def emit_step(self, *, phase: str, summary: str) -> None:
        self._queue.append(
            StepStreamEvent(
                phase=phase,
                summary=summary,
                sequence=self._next_sequence(),
            )
        )

    def emit_activity(self, *, phase: str, summary: str, detail: str = "") -> None:
        self._queue.append(
            ActivityStreamEvent(
                phase=phase,
                summary=summary,
                detail=detail,
                sequence=self._next_sequence(),
            )
        )

    def emit_tool(
        self,
        *,
        name: str,
        phase: str,
        detail: str = "",
        tool_call_id: str = "",
    ) -> None:
        elapsed_ms: int | None = None
        if phase == "running" and tool_call_id:
            self._tool_starts[tool_call_id] = time.monotonic()
        if phase in ("completed", "failed") and tool_call_id:
            started = self._tool_starts.pop(tool_call_id, None)
            if started is not None:
                elapsed_ms = int((time.monotonic() - started) * 1000)
        self._queue.append(
            ToolStreamEvent(
                name=name,
                phase=phase,
                detail=detail,
                tool_call_id=tool_call_id,
                sequence=self._next_sequence(),
                elapsed_ms=elapsed_ms,
            )
        )

    def emit_plan(self, *, phase: str, title: str, detail: str = "") -> None:
        self._queue.append(
            PlanStreamEvent(
                phase=phase,
                title=title,
                detail=detail,
                sequence=self._next_sequence(),
            )
        )

    def emit_text(self, text: str) -> None:
        if text:
            self._queue.append(
                TextStreamEvent(text=text, sequence=self._next_sequence())
            )

    def drain(self) -> list[StreamEvent]:
        items = list(self._queue)
        self._queue.clear()
        return items


def active_sink() -> LiveStreamSink | None:
    return _active_sink.get()


def set_active_sink(sink: LiveStreamSink | None) -> contextvars.Token:
    return _active_sink.set(sink)


def reset_active_sink(token: contextvars.Token) -> None:
    _active_sink.reset(token)


def drain_active_sink() -> Iterator[StreamEvent]:
    sink = active_sink()
    if sink is None:
        return iter(())
    return iter(sink.drain())

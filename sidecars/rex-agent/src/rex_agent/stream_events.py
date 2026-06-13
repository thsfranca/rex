"""Structured stream events for RunTurn (tool/step visibility)."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Union


@dataclass(frozen=True)
class TextStreamEvent:
    text: str
    sequence: int = 0


@dataclass(frozen=True)
class ToolStreamEvent:
    name: str
    phase: str
    detail: str = ""
    tool_call_id: str = ""
    sequence: int = 0
    elapsed_ms: int | None = None


@dataclass(frozen=True)
class StepStreamEvent:
    phase: str
    summary: str
    sequence: int = 0


@dataclass(frozen=True)
class PlanStreamEvent:
    phase: str
    title: str
    detail: str = ""
    sequence: int = 0


@dataclass(frozen=True)
class ActivityStreamEvent:
    phase: str
    summary: str
    detail: str = ""
    sequence: int = 0


StreamEvent = Union[
    TextStreamEvent,
    ToolStreamEvent,
    StepStreamEvent,
    PlanStreamEvent,
    ActivityStreamEvent,
]

DETAIL_MAX_CHARS = 240


def cap_detail(text: str, limit: int = DETAIL_MAX_CHARS) -> str:
    trimmed = (text or "").strip()
    if len(trimmed) <= limit:
        return trimmed
    return f"{trimmed[: limit - 3]}..."


def tool_detail_from_call(call: object) -> str:
    args = getattr(call, "args", None) or {}
    if not isinstance(args, dict):
        return ""
    path = args.get("path")
    if isinstance(path, str) and path.strip():
        return cap_detail(path.strip())
    command = args.get("command")
    if isinstance(command, str) and command.strip():
        return cap_detail(command.strip())
    query = args.get("query")
    if isinstance(query, str) and query.strip():
        return cap_detail(query.strip())
    return ""

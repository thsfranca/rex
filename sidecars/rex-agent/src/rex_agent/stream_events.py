"""Structured stream events for RunTurn (tool/step visibility)."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal, Union


@dataclass(frozen=True)
class TextStreamEvent:
    text: str


@dataclass(frozen=True)
class ToolStreamEvent:
    name: str
    phase: str
    detail: str = ""


@dataclass(frozen=True)
class StepStreamEvent:
    phase: str
    summary: str


@dataclass(frozen=True)
class PlanStreamEvent:
    phase: str
    title: str
    detail: str = ""


StreamEvent = Union[TextStreamEvent, ToolStreamEvent, StepStreamEvent, PlanStreamEvent]

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
    return cap_detail(str(args))

"""Structured subagent metrics for daemon correlation."""

from __future__ import annotations

import json


def log_subagent_event(
    *,
    subagent: str,
    step: int,
    event: str,
    turn_id: str = "",
    compaction: bool = False,
    extra: dict | None = None,
) -> None:
    payload = {
        "event": event,
        "subagent": subagent,
        "step": step,
        "compaction": compaction,
    }
    if turn_id:
        payload["turn_id"] = turn_id
    if extra:
        payload.update(extra)
    print(f"rex-agent metrics={json.dumps(payload, separators=(',', ':'))}")

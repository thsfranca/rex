"""Structured subagent metrics for daemon correlation."""

from __future__ import annotations

import json
import time

AGENT_LOOP_STUCK_CODE = "agent_loop_stuck"

CAP_TERMINAL_SOFT = "cap_soft_paused"
CAP_TERMINAL_HARD = "cap_hard"
CAP_TERMINAL_LOOP = "loop_circuit_breaker"
CAP_TERMINAL_COMPLETE = "complete"


def classify_loop_terminal(state: dict) -> str:
    if state.get("soft_cap_paused"):
        return CAP_TERMINAL_SOFT
    final = str(state.get("final_answer") or "")
    if AGENT_LOOP_STUCK_CODE in final:
        return CAP_TERMINAL_LOOP
    if "Stopped after" in final and "tool steps" in final:
        return CAP_TERMINAL_HARD
    if state.get("done"):
        return CAP_TERMINAL_COMPLETE
    return "unknown"


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


def log_turn_loop_metrics(
    state: dict,
    *,
    turn_id: str = "",
    started_at: float | None = None,
    first_productive_at: float | None = None,
) -> None:
    tool_steps = int(state.get("tool_steps") or 0)
    max_steps = int(state.get("max_steps") or 0)
    productive_ratio = (tool_steps / max_steps) if max_steps > 0 else 0.0
    payload = {
        "event": "turn_loop",
        "cap_terminal": classify_loop_terminal(state),
        "productive_steps_ratio": round(productive_ratio, 4),
        "policy_deny_retry_count": int(state.get("tool_error_count") or 0),
        "tool_steps": tool_steps,
        "max_steps": max_steps,
    }
    if turn_id:
        payload["turn_id"] = turn_id
    if started_at is not None and first_productive_at is not None:
        payload["time_to_first_productive_tool_ms"] = max(
            0, int((first_productive_at - started_at) * 1000)
        )
    print(f"rex-agent metrics={json.dumps(payload, separators=(',', ':'))}")


def monotonic_now() -> float:
    return time.monotonic()

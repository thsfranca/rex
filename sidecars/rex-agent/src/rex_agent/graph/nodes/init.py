"""Deterministic workspace-first initialization before first LLM inference."""

from __future__ import annotations

import re
import uuid

from langchain_core.messages import HumanMessage

from rex_agent.broker import BrokerClient
from rex_agent.config import deterministic_init_enabled
from rex_agent.graph.compaction import truncation_note
from rex_agent.graph.state import AgentState
from rex_agent.graph.stream_queue import append_step, append_tool
from rex_agent.metrics import log_subagent_event
from rex_agent.stream_events import cap_detail, tool_detail_from_call
from rex_agent.tools import (
    TOOL_LIST,
    TOOL_READ,
    ReadCache,
    ToolCall,
    execute_tool,
    format_tool_status,
    should_bill_tool_step,
)

_EXPLICIT_PATH = re.compile(
    r"(?:"
    r"@[\w./\\-]+"
    r"|(?:^|\s)(?:[\w.-]+/)+[\w.-]+\.(?:rs|py|md|json|toml|yaml|yml|tsx?|jsx?|go|sh)"
    r"|(?:^|\s)[\w.-]+\.(?:rs|py|md|json|toml|yaml|yml|tsx?|jsx?|go|sh)(?:\s|$)"
    r")",
    re.IGNORECASE | re.MULTILINE,
)

_README_INJECTED_MARKERS = (
    "<<tool_result:fs.read>>",
    "[context]",
    "# rex",
)


def prompt_has_explicit_file_reference(text: str) -> bool:
    return bool(_EXPLICIT_PATH.search(text or ""))


def prompt_already_has_readme_context(text: str) -> bool:
    lower = (text or "").lower()
    return any(marker in lower for marker in _README_INJECTED_MARKERS)


def should_run_deterministic_init(state: AgentState) -> bool:
    if not deterministic_init_enabled():
        return False
    mode = (state.get("mode") or "ask").strip().lower() or "ask"
    if mode != "ask":
        return False
    if state.get("workspace_explored"):
        return False
    goal_hint = state.get("goal_hint") or state.get("daemon_context") or ""
    if prompt_has_explicit_file_reference(goal_hint):
        return False
    if prompt_already_has_readme_context(state.get("daemon_context") or ""):
        return False
    return True


def init_workspace_node(state: AgentState, *, client: BrokerClient) -> dict:
    if not should_run_deterministic_init(state):
        return {}

    calls: list[ToolCall] = [ToolCall(tool=TOOL_READ, args={"path": "README.md"})]

    events = list(state.get("stream_events") or [])
    events = append_step(
        events,
        phase="running",
        summary="Deterministic workspace init (README + root listing)",
    )

    read_cache = state.get("read_cache") or ReadCache()
    new_messages: list[HumanMessage] = []
    stream_parts = list(state.get("stream_parts") or [])
    trunc_events = list(state.get("truncation_events") or [])
    workspace_explored = False
    batch_results: list[tuple[bool, str]] = []
    turn_id = state.get("turn_id", "") or "turn"

    def _run_call(index: int, call: ToolCall) -> None:
        nonlocal workspace_explored, events
        tool_call_id = f"{turn_id}:init:{index}:{call.tool}"
        detail = tool_detail_from_call(call)
        events = append_tool(
            events,
            name=call.tool,
            phase="running",
            detail=detail,
            tool_call_id=tool_call_id,
        )
        ok, result, truncated = execute_tool(
            client,
            call,
            state["mode"],
            read_cache=read_cache,
            goal_hint=state.get("goal_hint", ""),
        )
        batch_results.append((ok, result))
        status_line = format_tool_status(call, ok, result)
        result_detail = cap_detail(result if ok else status_line)
        events = append_tool(
            events,
            name=call.tool,
            phase="completed" if ok else "failed",
            detail=result_detail,
            tool_call_id=tool_call_id,
        )
        new_messages.append(HumanMessage(content=status_line, id=str(uuid.uuid4())))
        stream_parts.append(status_line)
        if ok and call.tool in (TOOL_READ, TOOL_LIST):
            workspace_explored = True
        if truncated and call.tool == TOOL_READ:
            path = str(call.args.get("path", ""))
            new_messages.append(truncation_note(path))
            trunc_events.append(path)
        log_subagent_event(
            subagent="viewer",
            step=1,
            event="deterministic_init",
            turn_id=turn_id,
            extra={"tool": call.tool, "ok": ok, "batch_index": index},
        )

    _run_call(0, calls[0])
    if not batch_results[-1][0]:
        fallback = ToolCall(tool=TOOL_READ, args={"path": "README"})
        calls.append(fallback)
        _run_call(1, fallback)

    list_call = ToolCall(tool=TOOL_LIST, args={"path": ""})
    calls.append(list_call)
    _run_call(len(calls) - 1, list_call)

    steps = 1 if should_bill_tool_step(batch_results) else 0
    error_count = 0 if should_bill_tool_step(batch_results) else state.get(
        "tool_error_count", 0
    )

    return {
        "tool_steps": steps,
        "tool_error_count": error_count,
        "workspace_explored": workspace_explored,
        "stream_parts": stream_parts,
        "stream_events": events,
        "messages": new_messages,
        "active_subagent": "viewer",
        "read_cache": read_cache,
        "truncation_events": trunc_events,
    }

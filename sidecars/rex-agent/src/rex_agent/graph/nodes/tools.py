"""Broker tool execution node."""

from __future__ import annotations

import uuid

from langchain_core.messages import HumanMessage

from rex_agent.broker import BrokerClient
from rex_agent.broker_chat_model import MAX_PARSE_RETRIES
from rex_agent.graph.compaction import truncation_note
from rex_agent.graph.nodes.orchestrator import classify_subagent_for_tool
from rex_agent.graph.state import AgentState
from rex_agent.graph.stream_queue import append_step, append_tool
from rex_agent.metrics import log_subagent_event
from rex_agent.stream_events import cap_detail, tool_detail_from_call
from rex_agent.tools import (
    BATCH_TRUNCATED_NOTE,
    TOOL_LIST,
    TOOL_READ,
    ReadCache,
    execute_tool,
    format_tool_status,
    is_policy_config_failure,
    should_bill_tool_step,
)

AGENT_LOOP_STUCK_CODE = "agent_loop_stuck"
AGENT_LOOP_STUCK_MESSAGE = (
    "Agent stopped after repeated blocked or invalid tool attempts "
    f"({AGENT_LOOP_STUCK_CODE}). Try rephrasing or narrowing the request."
)


def _limit_key_for_mode(mode: str) -> str:
    normalized = (mode or "ask").strip().lower() or "ask"
    if normalized == "ask":
        return "agent.max_tool_steps_ask"
    if normalized == "plan":
        return "agent.max_tool_steps_plan"
    return "agent.max_tool_steps"


def _terminal_cap_response(
    state: AgentState,
    *,
    calls: list,
    events: list,
    steps: int,
) -> dict:
    mode = (state.get("mode") or "ask").strip().lower() or "ask"
    limit_key = _limit_key_for_mode(mode)
    message = (
        f"Stopped after {state['max_steps']} tool steps ({limit_key}). "
        "Try a narrower request."
    )
    turn_id = state.get("turn_id", "") or "turn"
    call = calls[0]
    tool_call_id = f"{turn_id}:{steps + 1}:{call.tool}"
    events = append_tool(
        events,
        name=call.tool,
        phase="failed",
        detail="max tool steps exceeded",
        tool_call_id=tool_call_id,
    )
    return {
        "done": True,
        "final_answer": message,
        "stream_parts": state["stream_parts"] + [message],
        "stream_events": events,
        "tool_steps": steps,
        "pending_tools": [],
        "batch_truncated": False,
    }


def tools_node(state: AgentState, *, client: BrokerClient) -> dict:
    calls = list(state.get("pending_tools") or [])
    if not calls:
        return {}

    current_steps = state["tool_steps"]
    events = list(state.get("stream_events") or [])
    subagent = state.get("active_subagent", "orchestrator")
    if len(calls) == 1:
        summary = f"{subagent} invoking {calls[0].tool}"
    else:
        summary = f"{subagent} invoking {len(calls)} tools"
    events = append_step(events, phase="running", summary=summary)

    turn_id = state.get("turn_id", "") or "turn"

    if current_steps >= state["max_steps"]:
        return _terminal_cap_response(
            state, calls=calls, events=events, steps=current_steps
        )

    read_cache = state.get("read_cache") or ReadCache()
    new_messages: list[HumanMessage] = []
    stream_parts = list(state["stream_parts"])
    trunc_events = list(state.get("truncation_events") or [])
    workspace_explored = bool(state.get("workspace_explored"))
    batch_results: list[tuple[bool, str]] = []
    duplicate_hits = 0
    log_step = current_steps + 1

    for index, call in enumerate(calls):
        tool_call_id = f"{turn_id}:{log_step}:{index}:{call.tool}"
        detail = tool_detail_from_call(call)
        events = append_tool(
            events,
            name=call.tool,
            phase="running",
            detail=detail,
            tool_call_id=tool_call_id,
        )
        ok, result, truncated, exact_duplicate = execute_tool(
            client,
            call,
            state["mode"],
            read_cache=read_cache,
            goal_hint=state.get("goal_hint", ""),
        )
        if exact_duplicate:
            duplicate_hits += 1
        batch_results.append((ok and not exact_duplicate, result))
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
            subagent=classify_subagent_for_tool(call.tool),
            step=log_step,
            event="tool_execute",
            turn_id=state.get("turn_id", ""),
            extra={"tool": call.tool, "ok": ok, "batch_index": index},
        )

    if state.get("batch_truncated"):
        new_messages.append(
            HumanMessage(content=BATCH_TRUNCATED_NOTE, id=str(uuid.uuid4()))
        )
        stream_parts.append(BATCH_TRUNCATED_NOTE)

    last_subagent = classify_subagent_for_tool(calls[-1].tool)
    bill_step = should_bill_tool_step(batch_results)
    steps = current_steps + 1 if bill_step else current_steps
    error_count = state.get("tool_error_count", 0)
    if bill_step:
        error_count = 0
    elif duplicate_hits:
        error_count += duplicate_hits
    elif batch_results and all(
        not ok and is_policy_config_failure(result) for ok, result in batch_results
    ):
        error_count += 1

    if error_count >= MAX_PARSE_RETRIES:
        events = append_tool(
            events,
            name=calls[-1].tool,
            phase="failed",
            detail=AGENT_LOOP_STUCK_CODE,
            tool_call_id=f"{turn_id}:loop:{error_count}:{calls[-1].tool}",
        )
        return {
            "done": True,
            "final_answer": AGENT_LOOP_STUCK_MESSAGE,
            "stream_parts": stream_parts + [AGENT_LOOP_STUCK_MESSAGE],
            "stream_events": events,
            "tool_steps": steps,
            "tool_error_count": error_count,
            "workspace_explored": workspace_explored,
            "pending_tools": [],
            "batch_truncated": False,
            "messages": new_messages,
            "active_subagent": last_subagent,
            "read_cache": read_cache,
            "truncation_events": trunc_events,
        }

    return {
        "tool_steps": steps,
        "tool_error_count": error_count,
        "workspace_explored": workspace_explored,
        "stream_parts": stream_parts,
        "stream_events": events,
        "pending_tools": [],
        "batch_truncated": False,
        "messages": new_messages,
        "active_subagent": last_subagent,
        "read_cache": read_cache,
        "truncation_events": trunc_events,
    }

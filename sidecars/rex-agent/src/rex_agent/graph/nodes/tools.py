"""Broker tool execution node."""

from __future__ import annotations

import uuid

from langchain_core.messages import HumanMessage

from rex_agent.broker import BrokerClient
from rex_agent.graph.compaction import truncation_note
from rex_agent.graph.nodes.orchestrator import classify_subagent_for_tool
from rex_agent.graph.state import AgentState
from rex_agent.graph.stream_queue import append_step, append_tool
from rex_agent.metrics import log_subagent_event
from rex_agent.stream_events import cap_detail, tool_detail_from_call
from rex_agent.tools import (
    BATCH_TRUNCATED_NOTE,
    ReadCache,
    TOOL_READ,
    execute_tool,
    format_tool_status,
)


def _limit_key_for_mode(mode: str) -> str:
    normalized = (mode or "ask").strip().lower() or "ask"
    if normalized == "ask":
        return "agent.max_tool_steps_ask"
    return "agent.max_tool_steps"


def tools_node(state: AgentState, *, client: BrokerClient) -> dict:
    calls = list(state.get("pending_tools") or [])
    if not calls:
        return {}

    steps = state["tool_steps"] + 1
    events = list(state.get("stream_events") or [])
    subagent = state.get("active_subagent", "orchestrator")
    if len(calls) == 1:
        summary = f"{subagent} invoking {calls[0].tool}"
    else:
        summary = f"{subagent} invoking {len(calls)} tools"
    events = append_step(events, phase="running", summary=summary)

    turn_id = state.get("turn_id", "") or "turn"

    if steps > state["max_steps"]:
        mode = (state.get("mode") or "ask").strip().lower() or "ask"
        limit_key = _limit_key_for_mode(mode)
        message = (
            f"Stopped after {state['max_steps']} tool steps ({limit_key}). "
            "Try a narrower request."
        )
        call = calls[0]
        tool_call_id = f"{turn_id}:{steps}:{call.tool}"
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

    read_cache = state.get("read_cache") or ReadCache()
    new_messages: list[HumanMessage] = []
    stream_parts = list(state["stream_parts"])
    trunc_events = list(state.get("truncation_events") or [])

    for index, call in enumerate(calls):
        tool_call_id = f"{turn_id}:{steps}:{index}:{call.tool}"
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
        if truncated and call.tool == TOOL_READ:
            path = str(call.args.get("path", ""))
            new_messages.append(truncation_note(path))
            trunc_events.append(path)
        log_subagent_event(
            subagent=classify_subagent_for_tool(call.tool),
            step=steps,
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

    return {
        "tool_steps": steps,
        "stream_parts": stream_parts,
        "stream_events": events,
        "pending_tools": [],
        "batch_truncated": False,
        "messages": new_messages,
        "active_subagent": last_subagent,
        "read_cache": read_cache,
        "truncation_events": trunc_events,
    }

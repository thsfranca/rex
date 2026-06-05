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
from rex_agent.tools import ReadCache, execute_tool, format_tool_status


def tools_node(state: AgentState, *, client: BrokerClient) -> dict:
    call = state.get("pending_tool")
    if call is None:
        return {}

    steps = state["tool_steps"] + 1
    events = list(state.get("stream_events") or [])
    events = append_step(
        events,
        phase="running",
        summary=f"{state.get('active_subagent', 'agent')} invoking {call.tool}",
    )
    detail = tool_detail_from_call(call)
    events = append_tool(events, name=call.tool, phase="running", detail=detail)

    if steps > state["max_steps"]:
        message = (
            f"Stopped after {state['max_steps']} tool steps (agent.max_tool_steps). "
            "Try a narrower request."
        )
        events = append_tool(
            events, name=call.tool, phase="failed", detail="max tool steps exceeded"
        )
        return {
            "done": True,
            "final_answer": message,
            "stream_parts": state["stream_parts"] + [message],
            "stream_events": events,
            "tool_steps": steps,
            "pending_tool": None,
        }

    read_cache = state.get("read_cache") or ReadCache()
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
    )
    new_messages = [HumanMessage(content=status_line, id=str(uuid.uuid4()))]
    trunc_events = list(state.get("truncation_events") or [])
    if truncated and call.tool == "fs.read":
        path = str(call.args.get("path", ""))
        new_messages.append(truncation_note(path))
        trunc_events.append(path)

    subagent = classify_subagent_for_tool(call.tool)
    log_subagent_event(
        subagent=subagent,
        step=steps,
        event="tool_execute",
        turn_id=state.get("turn_id", ""),
        extra={"tool": call.tool, "ok": ok},
    )

    return {
        "tool_steps": steps,
        "stream_parts": state["stream_parts"] + [status_line],
        "stream_events": events,
        "pending_tool": None,
        "messages": new_messages,
        "active_subagent": subagent,
        "read_cache": read_cache,
        "truncation_events": trunc_events,
    }

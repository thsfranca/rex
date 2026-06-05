"""LLM nodes using RexBrokerChatModel."""

from __future__ import annotations

import json
import uuid
from typing import Any

from langchain_core.messages import AIMessage, BaseMessage, HumanMessage

from rex_agent.broker_chat_model import (
    MAX_PARSE_RETRIES,
    messages_to_prompt,
    parse_to_ai_message,
    stream_visible_text,
)
from rex_agent.graph.nodes.orchestrator import classify_subagent_for_tool
from rex_agent.graph.state import AgentState
from rex_agent.graph.stream_queue import append_plan, append_step
from rex_agent.metrics import log_subagent_event
from rex_agent.stream_events import cap_detail
from rex_agent.tools import ToolCall, parse_model_output, tools_for_mode


def _messages_for_subagent(state: AgentState) -> list[BaseMessage]:
    messages = list(state.get("messages") or [])
    if state.get("active_subagent") != "editor":
        return messages
    return [
        msg
        for msg in messages
        if "[tool fs.read" not in (msg.content if isinstance(msg.content, str) else "")
        and "[tool fs.list" not in (msg.content if isinstance(msg.content, str) else "")
    ]


def llm_node(state: AgentState, *, inference_fn: Any) -> dict:
    if state.get("done"):
        return {}

    subagent = state.get("active_subagent", "orchestrator")
    messages = _messages_for_subagent(state)
    prompt = messages_to_prompt(
        messages,
        state["mode"],
        state.get("daemon_context", ""),
        subagent=subagent,
        viewer_summary=state.get("viewer_summary", ""),
    )

    log_subagent_event(
        subagent=subagent,
        step=state.get("tool_steps", 0),
        event="llm_invoke",
        turn_id=state.get("turn_id", ""),
    )

    ok, raw_text = inference_fn(prompt, state["mode"], state.get("model", ""))
    if not ok:
        message = (
            "Inference failed. Check that the daemon is running and "
            "HTTP inference is configured."
        )
        return {
            "done": True,
            "final_answer": message,
            "stream_parts": state["stream_parts"] + [message],
        }

    ai, _ = parse_to_ai_message(raw_text, state["mode"])

    if ai.tool_calls:
        tc = ai.tool_calls[0]
        call = ToolCall(tool=str(tc.get("name", "")), args=dict(tc.get("args") or {}))
        active = classify_subagent_for_tool(call.tool)
        events = append_step(
            list(state.get("stream_events") or []),
            phase="running",
            summary=f"Routing to {active} for {call.tool}",
        )
        return {
            "messages": [AIMessage(content=raw_text, id=str(uuid.uuid4()))],
            "pending_tool": call,
            "active_subagent": active,
            "stream_events": events,
        }

    parsed = parse_model_output(raw_text, state["mode"])
    if parsed.kind == "clarify" and parsed.clarify_questions:
        detail = cap_detail(json.dumps(parsed.clarify_questions))
        events = append_plan(
            list(state.get("stream_events") or []),
            phase="clarify",
            title="Clarify",
            detail=detail,
        )
        summary = "Clarifying questions ready."
        return {
            "done": True,
            "final_answer": summary,
            "stream_parts": state["stream_parts"] + [summary],
            "stream_events": events,
        }

    if parsed.kind == "final":
        if parsed.plan is not None:
            title = parsed.answer or "Plan"
            detail = cap_detail(json.dumps(parsed.plan))
            events = append_plan(
                list(state.get("stream_events") or []),
                phase="ready",
                title=title,
                detail=detail,
            )
            return {
                "done": True,
                "final_answer": title,
                "stream_parts": state["stream_parts"] + [title],
                "stream_events": events,
            }
        visible = "".join(stream_visible_text(parsed.answer)) or parsed.answer
        return {
            "done": True,
            "final_answer": parsed.answer,
            "stream_parts": state["stream_parts"] + [visible],
        }

    if parsed.kind == "error":
        errors = state.get("tool_error_count", 0) + 1
        updates: dict = {
            "tool_error_count": errors,
            "messages": [
                AIMessage(content=raw_text, id=str(uuid.uuid4())),
                HumanMessage(content=parsed.message, id=str(uuid.uuid4())),
            ],
        }
        if errors >= MAX_PARSE_RETRIES or not tools_for_mode(state["mode"]):
            updates["done"] = True
            updates["final_answer"] = parsed.message
            updates["stream_parts"] = state["stream_parts"] + [parsed.message]
        return updates

    if parsed.kind == "tool" and parsed.tool_call is not None:
        active = classify_subagent_for_tool(parsed.tool_call.tool)
        events = append_step(
            list(state.get("stream_events") or []),
            phase="running",
            summary=f"Routing to {active} for {parsed.tool_call.tool}",
        )
        return {
            "messages": [AIMessage(content=raw_text, id=str(uuid.uuid4()))],
            "pending_tool": parsed.tool_call,
            "active_subagent": active,
            "stream_events": events,
        }

    return {}

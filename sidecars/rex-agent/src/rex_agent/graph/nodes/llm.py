"""LLM nodes using RexBrokerChatModel."""

from __future__ import annotations

import json
import uuid
from typing import Any

from langchain_core.messages import AIMessage, BaseMessage, HumanMessage

from rex_agent.broker import InferenceResult, is_interim_fallback
from rex_agent.broker_chat_model import (
    MAX_PARSE_RETRIES,
    messages_to_chat_messages,
    messages_to_prompt,
    route_inference_result,
    stream_visible_text,
)
from rex_agent.graph.nodes.orchestrator import classify_subagent_for_tool
from rex_agent.graph.state import AgentState
from rex_agent.graph.stream_queue import append_plan, append_step
from rex_agent.metrics import log_subagent_event
from rex_agent.stream_events import cap_detail
from rex_agent.tools import (
    ToolCall,
    normalize_tool_batch,
    tool_gate_from_state,
    tool_specs_for_subagent,
    tools_for_mode,
)


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


def _route_tool_batch(
    calls: list[ToolCall],
    *,
    raw_text: str,
    state: AgentState,
    truncated: bool = False,
) -> dict:
    active = state.get("active_subagent", "orchestrator")
    if calls:
        active = classify_subagent_for_tool(calls[0].tool)
    summary_tool = calls[0].tool if len(calls) == 1 else f"{len(calls)} tools"
    events = append_step(
        list(state.get("stream_events") or []),
        phase="running",
        summary=f"Routing to {active} for {summary_tool}",
    )
    updates: dict = {
        "messages": [AIMessage(content=raw_text, id=str(uuid.uuid4()))],
        "pending_tools": calls,
        "active_subagent": active,
        "stream_events": events,
        "batch_truncated": truncated,
    }
    return updates


def _batch_validation_error(
    message: str,
    *,
    raw_text: str,
    state: AgentState,
) -> dict:
    errors = state.get("tool_error_count", 0) + 1
    updates: dict = {
        "tool_error_count": errors,
        "messages": [
            AIMessage(content=raw_text, id=str(uuid.uuid4())),
            HumanMessage(content=message, id=str(uuid.uuid4())),
        ],
    }
    if errors >= MAX_PARSE_RETRIES or not tools_for_mode(
        state["mode"], gate=tool_gate_from_state(state)
    ):
        updates["done"] = True
        updates["final_answer"] = message
        updates["stream_parts"] = state["stream_parts"] + [message]
    return updates


def _resolve_tool_calls(
    calls: list[ToolCall],
    *,
    raw_text: str,
    state: AgentState,
) -> dict:
    subagent = state.get("active_subagent", "orchestrator")
    gate = tool_gate_from_state(state)
    normalized, error, truncated = normalize_tool_batch(
        calls,
        mode=state["mode"],
        subagent=subagent,
        gate=gate,
    )
    if error is not None or normalized is None:
        return _batch_validation_error(
            error or "Invalid tool batch.",
            raw_text=raw_text,
            state=state,
        )
    return _route_tool_batch(
        normalized,
        raw_text=raw_text,
        state=state,
        truncated=truncated,
    )


def _route_tool_call(
    call: ToolCall,
    *,
    raw_text: str,
    state: AgentState,
) -> dict:
    return _resolve_tool_calls([call], raw_text=raw_text, state=state)


def _invoke_broker_inference(
    inference_fn: Any,
    *,
    prompt: str,
    mode: str,
    model: str,
    chat_messages: list[Any],
    tool_specs: list[Any],
) -> InferenceResult:
    if tool_specs:
        result = inference_fn(
            prompt,
            mode,
            model,
            messages=chat_messages,
            tools=tool_specs,
        )
        if is_interim_fallback(result):
            return inference_fn(
                prompt,
                mode,
                model,
                messages=chat_messages,
                tools=[],
            )
        return result
    return inference_fn(
        prompt,
        mode,
        model,
        messages=chat_messages,
        tools=None,
    )


def _inference_failure_message(detail: str) -> str:
    base = (
        "Inference failed. Check that the daemon is running and "
        "HTTP inference is configured."
    )
    trimmed = detail.strip()
    if not trimmed:
        return base
    if len(trimmed) > 160:
        trimmed = trimmed[:157] + "..."
    return f"{base} {trimmed}"


def llm_node(state: AgentState, *, inference_fn: Any) -> dict:
    if state.get("done"):
        return {}

    subagent = state.get("active_subagent", "orchestrator")
    gate = tool_gate_from_state(state)
    messages = _messages_for_subagent(state)
    prompt = messages_to_prompt(
        messages,
        state["mode"],
        state.get("daemon_context", ""),
        subagent=subagent,
        viewer_summary=state.get("viewer_summary", ""),
        gate=gate,
        injected_files=list(state.get("injected_files") or []),
    )
    chat_messages = messages_to_chat_messages(
        messages,
        state["mode"],
        state.get("daemon_context", ""),
        subagent=subagent,
        viewer_summary=state.get("viewer_summary", ""),
        gate=gate,
        injected_files=list(state.get("injected_files") or []),
    )
    tool_specs = tool_specs_for_subagent(subagent, state["mode"], gate=gate)

    log_subagent_event(
        subagent=subagent,
        step=state.get("tool_steps", 0),
        event="llm_invoke",
        turn_id=state.get("turn_id", ""),
    )

    result = _invoke_broker_inference(
        inference_fn,
        prompt=prompt,
        mode=state["mode"],
        model=state.get("model", ""),
        chat_messages=chat_messages,
        tool_specs=tool_specs,
    )
    if not result.ok:
        message = _inference_failure_message(result.error)
        return {
            "done": True,
            "final_answer": message,
            "stream_parts": state["stream_parts"] + [message],
        }

    raw_text = result.effective_text()
    ai, parsed = route_inference_result(result, state["mode"])

    if ai.tool_calls:
        calls = [
            ToolCall(tool=str(tc.get("name", "")), args=dict(tc.get("args") or {}))
            for tc in ai.tool_calls
        ]
        return _resolve_tool_calls(calls, raw_text=raw_text, state=state)

    if parsed is None:
        return {}

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
        if errors >= MAX_PARSE_RETRIES or not tools_for_mode(
        state["mode"], gate=tool_gate_from_state(state)
    ):
            updates["done"] = True
            updates["final_answer"] = parsed.message
            updates["stream_parts"] = state["stream_parts"] + [parsed.message]
        return updates

    if parsed.kind == "tool" and parsed.tool_call is not None:
        return _route_tool_call(
            parsed.tool_call, raw_text=raw_text, state=state
        )

    return {}

"""LangGraph agent package: Orchestrator / Viewer / Editor topology."""

from __future__ import annotations

import contextvars
from collections.abc import Iterator
from typing import Any

from langgraph.graph import END, StateGraph

from rex_agent.broker import BrokerClient, InferenceResult, legacy_inference_result
from rex_agent.broker_chat_model import stream_visible_text
from rex_agent.config import compaction_enabled
from rex_agent.graph.compaction import compact_state
from rex_agent.graph.nodes.init import init_workspace_node
from rex_agent.graph.nodes.llm import llm_node
from rex_agent.graph.nodes.orchestrator import (
    route_after_editor,
    route_after_orchestrator,
    route_after_tools,
    route_after_viewer,
)
from rex_agent.graph.nodes.tools import tools_node
from rex_agent.graph.state import AgentState
from rex_agent.graph.stream_sink import (
    LiveStreamSink,
    reset_active_sink,
    set_active_sink,
)
from rex_agent.metrics import log_turn_loop_metrics, monotonic_now
from rex_agent.stream_events import ActivityStreamEvent, StreamEvent, TextStreamEvent
from rex_agent.tools import ReadCache, tool_gate_from_state, tools_for_mode

_inference_fn: Any | None = None
_active_client: contextvars.ContextVar[BrokerClient | None] = contextvars.ContextVar(
    "rex_agent_broker_client", default=None
)

_REACT: Any | None = None
_ASK: Any | None = None


def set_inference_fn(fn: Any | None) -> None:
    global _inference_fn
    _inference_fn = fn


def _client() -> BrokerClient:
    client = _active_client.get()
    if client is None:
        raise RuntimeError("broker client not set for graph execution")
    return client


def _call_inference(
    prompt: str,
    mode: str,
    model: str,
    *,
    messages: list | None = None,
    tools: list | None = None,
) -> InferenceResult:
    if _inference_fn is not None:
        try:
            ret = _inference_fn(
                prompt,
                mode,
                model,
                messages=messages,
                tools=tools,
            )
        except TypeError:
            ret = _inference_fn(prompt, mode, model)
        if isinstance(ret, InferenceResult):
            return ret
        ok, text = ret
        return legacy_inference_result(ok, text)
    return _client().inference(
        prompt, mode, model, messages=messages, tools=tools
    )


def _orchestrator_node(state: AgentState) -> dict:
    return llm_node(
        {**state, "active_subagent": "orchestrator"}, inference_fn=_call_inference
    )


def _viewer_node(state: AgentState) -> dict:
    return llm_node(
        {**state, "active_subagent": "viewer"}, inference_fn=_call_inference
    )


def _editor_node(state: AgentState) -> dict:
    return llm_node(
        {**state, "active_subagent": "editor"}, inference_fn=_call_inference
    )


def _tools_wrapper(state: AgentState) -> dict:
    return tools_node(state, client=_client())


def _init_workspace_wrapper(state: AgentState) -> dict:
    return init_workspace_node(state, client=_client())


def _compaction_node(state: AgentState) -> dict:
    if not compaction_enabled():
        return {}
    return compact_state(state)


def _compile_react_graph() -> Any:
    graph: StateGraph = StateGraph(AgentState)
    graph.add_node("init_workspace", _init_workspace_wrapper)
    graph.add_node("orchestrator", _orchestrator_node)
    graph.add_node("viewer", _viewer_node)
    graph.add_node("editor", _editor_node)
    graph.add_node("tools", _tools_wrapper)
    graph.add_node("compact", _compaction_node)

    graph.set_entry_point("init_workspace")
    graph.add_edge("init_workspace", "orchestrator")
    graph.add_conditional_edges(
        "orchestrator",
        route_after_orchestrator,
        {"tools": "tools", "orchestrator": "orchestrator", "end": END},
    )
    graph.add_conditional_edges(
        "viewer",
        route_after_viewer,
        {"tools": "tools", "viewer": "viewer", "end": END},
    )
    graph.add_conditional_edges(
        "editor",
        route_after_editor,
        {"tools": "tools", "editor": "editor", "end": END},
    )
    graph.add_edge("tools", "compact")
    graph.add_conditional_edges(
        "compact",
        route_after_tools,
        {
            "orchestrator": "orchestrator",
            "viewer": "viewer",
            "editor": "editor",
            "end": END,
        },
    )
    return graph.compile()


def _compile_ask_graph() -> Any:
    graph: StateGraph = StateGraph(AgentState)
    graph.add_node("llm", _orchestrator_node)
    graph.set_entry_point("llm")
    graph.add_edge("llm", END)
    return graph.compile()


def _react_graph() -> Any:
    global _REACT
    if _REACT is None:
        _REACT = _compile_react_graph()
    return _REACT


def _ask_graph() -> Any:
    global _ASK
    if _ASK is None:
        _ASK = _compile_ask_graph()
    return _ASK


def _reset_graphs() -> None:
    global _REACT, _ASK
    _REACT = None
    _ASK = None


def _initial_state(
    prompt: str,
    mode: str,
    model: str,
    turn_id: str,
    injected_files: list[str] | None = None,
    harness_session_id: str = "",
) -> AgentState:
    normalized = (mode or "ask").strip().lower() or "ask"
    return AgentState(
        daemon_context=prompt,
        messages=[],
        mode=normalized,
        model=model or "",
        turn_id=turn_id,
        active_subagent="orchestrator",
        viewer_summary="",
        tool_steps=0,
        tool_error_count=0,
        max_steps=0,
        truncation_events=[],
        stream_parts=[],
        stream_events=[],
        final_answer="",
        done=False,
        pending_tools=[],
        batch_truncated=False,
        read_cache=ReadCache(),
        goal_hint=prompt[:500],
        workspace_explored=False,
        soft_cap_continued=False,
        injected_files=list(injected_files or []),
        harness_session_id=harness_session_id or "",
    )


def _graph_for_state(state: AgentState) -> Any:
    if tools_for_mode(state["mode"], gate=tool_gate_from_state(state)):
        return _react_graph()
    return _ask_graph()


def _run_graph_stream(
    state: AgentState,
    sink: LiveStreamSink,
    final_box: list[AgentState],
) -> Iterator[StreamEvent]:
    current: AgentState = state
    graph = _graph_for_state(state)
    for update in graph.stream(state, stream_mode="updates"):
        yield from _yield_visible_events(sink.drain())
        for partial in update.values():
            if isinstance(partial, dict):
                current = {**current, **partial}
        if current.get("done"):
            break
    yield from _yield_visible_events(sink.drain())
    final_box[0] = current


def _emit_final_text(final: AgentState, emitted_text: bool) -> Iterator[StreamEvent]:
    answer = final.get("final_answer") or ""
    parts = list(final.get("stream_parts") or [])
    if not answer and parts:
        answer = "".join(p for p in parts if p.strip())
    if not emitted_text and answer:
        for segment in stream_visible_text(answer):
            if segment:
                yield TextStreamEvent(text=segment)
    elif not emitted_text and parts:
        for part in parts:
            for segment in stream_visible_text(part):
                if segment:
                    yield TextStreamEvent(text=segment)


def _invoke(state: AgentState) -> AgentState:
    if tools_for_mode(state["mode"], gate=tool_gate_from_state(state)):
        return _react_graph().invoke(state)
    return _ask_graph().invoke(state)


def run_turn(
    prompt: str,
    mode: str,
    model: str,
    turn_id: str = "",
) -> tuple[str, list[str]]:
    state = _initial_state(prompt, mode, model, turn_id)
    with BrokerClient(
        turn_id=turn_id or None,
        harness_session_id=state.get("harness_session_id") or None,
    ) as client:
        token = _active_client.set(client)
        try:
            final = _invoke(state)
        finally:
            _active_client.reset(token)
    answer = final.get("final_answer") or ""
    parts = list(final.get("stream_parts") or [])
    if not answer and parts:
        answer = "".join(p for p in parts if p.strip())
    if not answer:
        answer = "No response from agent."
    return answer, parts


def _yield_visible_events(events: list[StreamEvent]) -> Iterator[StreamEvent]:
    for event in events:
        if isinstance(event, TextStreamEvent):
            for segment in stream_visible_text(event.text):
                if segment:
                    yield TextStreamEvent(text=segment, sequence=event.sequence)
        else:
            yield event


def stream_turn(
    prompt: str,
    mode: str,
    model: str,
    turn_id: str = "",
    injected_files: list[str] | None = None,
    harness_session_id: str = "",
) -> Iterator[StreamEvent]:
    state = _initial_state(
        prompt, mode, model, turn_id, injected_files, harness_session_id
    )
    yield from _stream_agent_state(state, turn_id)


def stream_continue_turn(
    continue_token: str, turn_id: str = ""
) -> Iterator[StreamEvent]:
    yield ActivityStreamEvent(
        phase="failed",
        summary="ContinueTurn is deprecated (R069)",
        detail="continue_token",
    )


def _stream_agent_state(state: AgentState, turn_id: str) -> Iterator[StreamEvent]:
    sink = LiveStreamSink(turn_id=turn_id)
    sink.emit_step(phase="running", summary="Agent turn started")
    emitted_text = False
    final_box: list[AgentState] = [state]
    started_at = monotonic_now()
    first_productive_at: float | None = None

    with BrokerClient(
        turn_id=turn_id or None,
        harness_session_id=state.get("harness_session_id") or None,
    ) as client:
        client_token = _active_client.set(client)
        sink_token = set_active_sink(sink)
        try:
            for event in _run_graph_stream(state, sink, final_box):
                if isinstance(event, TextStreamEvent):
                    emitted_text = True
                yield event
                if (
                    first_productive_at is None
                    and int(final_box[0].get("tool_steps") or 0) > 0
                ):
                    first_productive_at = monotonic_now()
        finally:
            reset_active_sink(sink_token)
            _active_client.reset(client_token)

    log_turn_loop_metrics(
        final_box[0],
        turn_id=turn_id,
        started_at=started_at,
        first_productive_at=first_productive_at,
    )
    yield from _emit_final_text(final_box[0], emitted_text)


def run_turn_with_events(
    prompt: str,
    mode: str,
    model: str,
    turn_id: str = "",
) -> tuple[str, list[str], list[StreamEvent]]:
    state = _initial_state(prompt, mode, model, turn_id)
    with BrokerClient(
        turn_id=turn_id or None,
        harness_session_id=state.get("harness_session_id") or None,
    ) as client:
        token = _active_client.set(client)
        try:
            final = _invoke(state)
        finally:
            _active_client.reset(token)
    answer = final.get("final_answer") or ""
    parts = list(final.get("stream_parts") or [])
    events = list(final.get("stream_events") or [])
    if not answer and parts:
        answer = "".join(p for p in parts if p.strip())
    if not answer:
        answer = "No response from agent."
    visible_parts: list[StreamEvent] = []
    for part in parts:
        visible = "".join(stream_visible_text(part)) or part
        if visible.strip():
            visible_parts.append(TextStreamEvent(text=visible))
    if visible_parts:
        events = events + visible_parts
    elif answer:
        visible = "".join(stream_visible_text(answer)) or answer
        if visible.strip():
            events = events + [TextStreamEvent(text=visible)]
    return answer, parts, events

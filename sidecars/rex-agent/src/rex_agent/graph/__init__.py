"""LangGraph agent package: Orchestrator / Viewer / Editor topology."""

from __future__ import annotations

import contextvars
from collections.abc import Iterator
from typing import Any

from langgraph.graph import END, StateGraph

from rex_agent.broker import BrokerClient
from rex_agent.broker_chat_model import stream_visible_text
from rex_agent.config import max_tool_steps
from rex_agent.graph.compaction import compact_state
from rex_agent.graph.nodes.llm import llm_node
from rex_agent.graph.nodes.orchestrator import (
    route_after_editor,
    route_after_orchestrator,
    route_after_tools,
    route_after_viewer,
)
from rex_agent.graph.nodes.tools import tools_node
from rex_agent.graph.state import AgentState
from rex_agent.tools import ReadCache, tools_for_mode

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


def _call_inference(prompt: str, mode: str, model: str) -> tuple[bool, str]:
    if _inference_fn is not None:
        return _inference_fn(prompt, mode, model)
    return _client().inference(prompt, mode, model)


def _orchestrator_node(state: AgentState) -> dict:
    return llm_node({**state, "active_subagent": "orchestrator"}, inference_fn=_call_inference)


def _viewer_node(state: AgentState) -> dict:
    return llm_node({**state, "active_subagent": "viewer"}, inference_fn=_call_inference)


def _editor_node(state: AgentState) -> dict:
    return llm_node({**state, "active_subagent": "editor"}, inference_fn=_call_inference)


def _tools_wrapper(state: AgentState) -> dict:
    return tools_node(state, client=_client())


def _compaction_node(state: AgentState) -> dict:
    return compact_state(state)


def _compile_react_graph() -> Any:
    graph: StateGraph = StateGraph(AgentState)
    graph.add_node("orchestrator", _orchestrator_node)
    graph.add_node("viewer", _viewer_node)
    graph.add_node("editor", _editor_node)
    graph.add_node("tools", _tools_wrapper)
    graph.add_node("compact", _compaction_node)

    graph.set_entry_point("orchestrator")
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
        {"orchestrator": "orchestrator", "viewer": "viewer", "editor": "editor", "end": END},
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


def _initial_state(prompt: str, mode: str, model: str, turn_id: str) -> AgentState:
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
        max_steps=max_tool_steps(),
        truncation_events=[],
        stream_parts=[],
        final_answer="",
        done=False,
        pending_tool=None,
        read_cache=ReadCache(),
        goal_hint=prompt[:500],
    )


def _invoke(state: AgentState) -> AgentState:
    if tools_for_mode(state["mode"]):
        return _react_graph().invoke(state)
    return _ask_graph().invoke(state)


def run_turn(
    prompt: str,
    mode: str,
    model: str,
    turn_id: str = "",
) -> tuple[str, list[str]]:
    state = _initial_state(prompt, mode, model, turn_id)
    with BrokerClient(turn_id=turn_id or None) as client:
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


def stream_turn(
    prompt: str,
    mode: str,
    model: str,
    turn_id: str = "",
) -> Iterator[str]:
    answer, parts = run_turn(prompt, mode, model, turn_id)
    emitted = False
    for part in parts:
        for segment in stream_visible_text(part):
            if segment:
                emitted = True
                yield segment
    if not emitted and answer:
        for segment in stream_visible_text(answer):
            if segment:
                yield segment

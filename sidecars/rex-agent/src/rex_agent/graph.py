"""LangGraph ReAct loops (broker-only LLM and tools)."""

from __future__ import annotations

import contextvars
from collections.abc import Iterator
from typing import Annotated, Any, Optional, TypedDict

from langgraph.graph import END, StateGraph

from rex_agent.broker import BrokerClient
from rex_agent.config import max_tool_steps
from rex_agent.llm import Conversation, build_initial_conversation
from rex_agent.tools import (
    ToolCall,
    execute_tool,
    format_tool_status,
    parse_model_output,
    tools_for_mode,
)

_inference_fn: Any | None = None
_active_client: contextvars.ContextVar[BrokerClient | None] = contextvars.ContextVar(
    "rex_agent_broker_client", default=None
)


def set_inference_fn(fn: Any | None) -> None:
    global _inference_fn
    _inference_fn = fn


class AgentState(TypedDict):
    mode: str
    model: str
    turn_id: str
    conversation: Conversation
    tool_steps: int
    max_steps: int
    stream_parts: Annotated[list[str], lambda left, right: left + right]
    final_answer: str
    done: bool
    parse_retries: int
    pending_tool: Optional[ToolCall]


def _client() -> BrokerClient:
    client = _active_client.get()
    if client is None:
        raise RuntimeError("broker client not set for graph execution")
    return client


def _call_inference(prompt: str, mode: str, model: str) -> tuple[bool, str]:
    if _inference_fn is not None:
        return _inference_fn(prompt, mode, model)
    return _client().inference(prompt, mode, model)


def _llm_node(state: AgentState) -> AgentState:
    if state["done"]:
        return state
    prompt = state["conversation"].to_prompt()
    ok, text = _call_inference(prompt, state["mode"], state["model"])
    if not ok:
        message = (
            "Inference failed. Check that the daemon is running and HTTP inference is configured."
        )
        return {
            **state,
            "done": True,
            "final_answer": message,
            "stream_parts": state["stream_parts"] + [message],
        }

    parsed = parse_model_output(text, state["mode"])
    if parsed.kind == "final":
        return {
            **state,
            "done": True,
            "final_answer": parsed.answer,
            "stream_parts": state["stream_parts"] + [parsed.answer],
        }
    if parsed.kind == "error":
        retries = state["parse_retries"] + 1
        if retries <= 1 and tools_for_mode(state["mode"]):
            conv = state["conversation"]
            conv.append("assistant", text)
            conv.append("user", parsed.message)
            return {**state, "parse_retries": retries, "conversation": conv}
        return {
            **state,
            "done": True,
            "final_answer": parsed.message,
            "stream_parts": state["stream_parts"] + [parsed.message],
        }
    if parsed.kind == "tool" and parsed.tool_call is not None:
        state["conversation"].append("assistant", text)
        return {**state, "pending_tool": parsed.tool_call}
    return state


def _tools_node(state: AgentState) -> AgentState:
    call = state.get("pending_tool")
    if call is None:
        return state
    steps = state["tool_steps"] + 1
    if steps > state["max_steps"]:
        message = (
            f"Stopped after {state['max_steps']} tool steps (agent.max_tool_steps). "
            "Try a narrower request."
        )
        return {
            **state,
            "done": True,
            "final_answer": message,
            "stream_parts": state["stream_parts"] + [message],
            "tool_steps": steps,
            "pending_tool": None,
        }

    ok, result = execute_tool(_client(), call, state["mode"])
    status_line = format_tool_status(call, ok, result)
    conv = state["conversation"]
    conv.append("tool", status_line)
    return {
        **state,
        "tool_steps": steps,
        "stream_parts": state["stream_parts"] + [status_line],
        "pending_tool": None,
    }


def _route_after_llm(state: AgentState) -> str:
    if state.get("done"):
        return "end"
    if state.get("pending_tool") is not None:
        return "tools"
    return "end"


def _compile_react_graph() -> Any:
    graph: StateGraph = StateGraph(AgentState)
    graph.add_node("llm", _llm_node)
    graph.add_node("tools", _tools_node)
    graph.set_entry_point("llm")
    graph.add_conditional_edges("llm", _route_after_llm, {"tools": "tools", "end": END})
    graph.add_edge("tools", "llm")
    return graph.compile()


def _compile_ask_graph() -> Any:
    graph: StateGraph = StateGraph(AgentState)
    graph.add_node("llm", _llm_node)
    graph.set_entry_point("llm")
    graph.add_edge("llm", END)
    return graph.compile()


_REACT: Any | None = None
_ASK: Any | None = None


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


def _initial_state(
    prompt: str,
    mode: str,
    model: str,
    turn_id: str,
) -> AgentState:
    normalized = (mode or "ask").strip().lower() or "ask"
    return AgentState(
        mode=normalized,
        model=model or "",
        turn_id=turn_id,
        conversation=build_initial_conversation(prompt, normalized),
        tool_steps=0,
        max_steps=max_tool_steps(),
        stream_parts=[],
        final_answer="",
        done=False,
        parse_retries=0,
        pending_tool=None,
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
    """Run the mode-appropriate graph; returns (final_answer, stream_parts)."""
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
    """Yield user-visible segments."""
    answer, parts = run_turn(prompt, mode, model, turn_id)
    emitted = False
    for part in parts:
        if part:
            emitted = True
            yield part
    if not emitted and answer:
        yield answer

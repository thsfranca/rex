"""Orchestrator routing helpers."""

from __future__ import annotations

from rex_agent.broker_chat_model import MAX_PARSE_RETRIES
from rex_agent.graph.state import AgentState
from rex_agent.tools import (
    TOOL_EXEC,
    TOOL_LIST,
    TOOL_READ,
    TOOL_WEB_SEARCH,
    TOOL_WORKSPACE_SEARCH,
    TOOL_WRITE,
    tool_gate_from_state,
    tools_for_mode,
)


def classify_subagent_for_tool(tool: str) -> str:
    if tool in (TOOL_READ, TOOL_LIST, TOOL_WEB_SEARCH, TOOL_WORKSPACE_SEARCH):
        return "viewer"
    if tool in (TOOL_WRITE, TOOL_EXEC):
        return "editor"
    return "orchestrator"


def _route_after_llm(state: AgentState, node: str) -> str:
    if state.get("done"):
        return "end"
    if state.get("pending_tools"):
        return "tools"
    errors = state.get("tool_error_count", 0)
    if (
        errors > 0
        and errors <= MAX_PARSE_RETRIES
        and tools_for_mode(state["mode"], gate=tool_gate_from_state(state))
    ):
        return node
    return "end"


def route_after_orchestrator(state: AgentState) -> str:
    return _route_after_llm(state, "orchestrator")


def route_after_viewer(state: AgentState) -> str:
    return _route_after_llm(state, "viewer")


def route_after_editor(state: AgentState) -> str:
    return _route_after_llm(state, "editor")


def route_after_tools(state: AgentState) -> str:
    if state.get("done"):
        return "end"
    subagent = state.get("active_subagent", "orchestrator")
    if subagent == "viewer":
        return "viewer"
    if subagent == "editor":
        return "editor"
    return "orchestrator"

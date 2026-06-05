"""Agent graph state schema."""

from __future__ import annotations

from typing import Annotated, Optional, TypedDict

from langchain_core.messages import BaseMessage
from langgraph.graph.message import add_messages

from rex_agent.graph.stream_queue import StreamEventList
from rex_agent.tools import ReadCache, ToolCall


class AgentState(TypedDict):
    daemon_context: str
    messages: Annotated[list[BaseMessage], add_messages]
    mode: str
    model: str
    turn_id: str
    active_subagent: str
    viewer_summary: str
    tool_steps: int
    tool_error_count: int
    max_steps: int
    truncation_events: list[str]
    stream_parts: Annotated[list[str], lambda left, right: left + right]
    stream_events: StreamEventList
    final_answer: str
    done: bool
    pending_tool: Optional[ToolCall]
    read_cache: ReadCache
    goal_hint: str

"""Intra-turn RemoveMessage compaction (R029)."""

from __future__ import annotations

import uuid

from langchain_core.messages import BaseMessage, HumanMessage, RemoveMessage

from rex_agent.config import compaction_suffix_fraction, max_tool_result_bytes
from rex_agent.graph.state import AgentState
from rex_agent.metrics import log_subagent_event


def _scratch_byte_size(messages: list[BaseMessage]) -> int:
    total = 0
    for msg in messages:
        content = msg.content
        if isinstance(content, str):
            total += len(content.encode("utf-8"))
    return total


def _summarize_tool_messages(messages: list[BaseMessage]) -> str:
    snippets: list[str] = []
    for msg in messages:
        content = msg.content if isinstance(msg.content, str) else str(msg.content)
        if "[tool " in content:
            lines = content.strip().splitlines()
            if lines:
                snippets.append(lines[0][:120])
    return (
        "Viewer findings: " + "; ".join(snippets[:8])
        if snippets
        else "No tool results to summarize."
    )


def compact_state(state: AgentState) -> dict:
    messages = list(state.get("messages") or [])
    if not messages:
        return {}

    threshold = int(max_tool_result_bytes() * compaction_suffix_fraction())
    if _scratch_byte_size(messages) <= threshold:
        return {}

    summary = _summarize_tool_messages(messages)
    removals = [
        RemoveMessage(id=msg.id)
        for msg in messages
        if getattr(msg, "id", None)
        and "[tool " in (msg.content if isinstance(msg.content, str) else "")
    ]
    updates: dict = {"viewer_summary": summary}
    if removals:
        updates["messages"] = removals
        log_subagent_event(
            subagent=state.get("active_subagent", "orchestrator"),
            step=state.get("tool_steps", 0),
            event="compaction",
            turn_id=state.get("turn_id", ""),
            compaction=True,
        )
    return updates


def truncation_note(path: str) -> HumanMessage:
    return HumanMessage(
        id=str(uuid.uuid4()),
        content=(
            f"[system note] Broker truncated read result for {path}. "
            "Re-read with a narrower path if you need more."
        ),
    )

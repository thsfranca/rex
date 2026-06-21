"""Soft cap removed (R069) — tools node no longer pauses at thresholds."""

from __future__ import annotations

from unittest.mock import MagicMock

from rex_agent.graph.nodes.tools import tools_node
from rex_agent.tools import ReadCache, ToolCall


def _tool_state(*, steps: int) -> dict:
    return {
        "mode": "ask",
        "turn_id": "turn-soft",
        "tool_steps": steps,
        "max_steps": 0,
        "soft_cap_continued": False,
        "stream_parts": [],
        "stream_events": [],
        "pending_tools": [ToolCall(tool="fs.read", args={"path": "README.md"})],
        "read_cache": ReadCache(),
        "tool_error_count": 0,
        "batch_truncated": False,
        "goal_hint": "explore",
        "active_subagent": "viewer",
        "workspace_explored": True,
    }


def test_tools_node_runs_past_former_soft_threshold() -> None:
    client = MagicMock()
    client.read_file.return_value = (True, "body")
    result = tools_node(_tool_state(steps=10), client=client)
    assert not result.get("soft_cap_paused")
    assert not result.get("done")
    client.read_file.assert_called_once()


def test_tools_node_runs_past_former_hard_cap() -> None:
    client = MagicMock()
    client.read_file.return_value = (True, "body")
    result = tools_node(_tool_state(steps=100), client=client)
    assert "max tool steps" not in str(result.get("final_answer", ""))
    client.read_file.assert_called_once()

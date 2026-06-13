"""Soft cap pause and ContinueTurn tests (R063)."""

from __future__ import annotations

from unittest.mock import MagicMock, patch

from rex_agent.config import soft_cap_threshold
from rex_agent.graph.checkpoints import clear_checkpoints_for_tests, consume_soft_cap_checkpoint
from rex_agent.graph.nodes.tools import SOFT_CAP_PAUSE_PHASE, tools_node
from rex_agent.tools import ReadCache, ToolCall


def _tool_state(*, steps: int, max_steps: int = 15, continued: bool = False) -> dict:
    return {
        "mode": "ask",
        "turn_id": "turn-soft",
        "tool_steps": steps,
        "max_steps": max_steps,
        "soft_cap_continued": continued,
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


def test_soft_cap_threshold_ask_default() -> None:
    assert soft_cap_threshold(15) == 10


@patch("rex_agent.graph.nodes.tools.soft_cap_enabled", return_value=True)
def test_tools_node_pauses_at_soft_threshold(_enabled: MagicMock) -> None:
    clear_checkpoints_for_tests()
    client = MagicMock()
    result = tools_node(_tool_state(steps=10), client=client)
    assert result.get("soft_cap_paused") is True
    assert result.get("done") is True
    events = result.get("stream_events") or []
    activity = [event for event in events if getattr(event, "phase", "") == SOFT_CAP_PAUSE_PHASE]
    assert activity
    token = activity[0].detail
    assert token
    assert consume_soft_cap_checkpoint(token) is not None
    client.read_file.assert_not_called()


@patch("rex_agent.graph.nodes.tools.soft_cap_enabled", return_value=False)
def test_tools_node_skips_soft_cap_when_disabled(_enabled: MagicMock) -> None:
    client = MagicMock()
    client.read_file.return_value = (True, "body")
    result = tools_node(_tool_state(steps=10), client=client)
    assert not result.get("soft_cap_paused")
    client.read_file.assert_called_once()

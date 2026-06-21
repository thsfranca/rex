"""Advisory intent helpers (R067 / R070)."""

from __future__ import annotations

from rex_agent.advisory_intent import (
    daemon_context_has_priority_markers,
    matches_advisory_intent,
)
from rex_agent.graph.nodes.init import should_run_deterministic_init


def test_matches_advisory_intent_short_prompt() -> None:
    assert matches_advisory_intent("What should we do next?")


def test_skips_unrelated_prompt() -> None:
    assert not matches_advisory_intent("hello")


def test_daemon_context_priority_markers() -> None:
    assert daemon_context_has_priority_markers("[context]\ndocs/ROADMAP.md row")


def test_should_skip_deterministic_init_for_advisory_prompt() -> None:
    state = {
        "mode": "ask",
        "goal_hint": "What should we do next?",
        "daemon_context": "",
        "workspace_explored": False,
        "injected_files": [],
    }
    with patch_deterministic_init_enabled(True):
        assert should_run_deterministic_init(state) is False


def patch_deterministic_init_enabled(enabled: bool):
    from unittest.mock import patch

    return patch(
        "rex_agent.graph.nodes.init.deterministic_init_enabled",
        return_value=enabled,
    )

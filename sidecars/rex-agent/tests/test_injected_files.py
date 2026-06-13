"""R065 injected_files manifest on RunTurn."""

from __future__ import annotations

from unittest.mock import MagicMock, patch

from rex_agent.graph.nodes import init as init_node
from rex_agent.graph.state import AgentState
from rex_agent.tools import ReadCache, injected_files_system_note


def _ask_state(**overrides: object) -> AgentState:
    base: AgentState = AgentState(
        daemon_context="What is rex?",
        messages=[],
        mode="ask",
        model="",
        turn_id="turn-injected",
        active_subagent="orchestrator",
        viewer_summary="",
        tool_steps=0,
        tool_error_count=0,
        max_steps=15,
        truncation_events=[],
        stream_parts=[],
        stream_events=[],
        final_answer="",
        done=False,
        pending_tools=[],
        batch_truncated=False,
        read_cache=ReadCache(),
        goal_hint="What is rex?",
        workspace_explored=False,
        soft_cap_continued=False,
        injected_files=[],
    )
    return {**base, **overrides}  # type: ignore[misc]


def test_injected_files_system_note_lists_paths() -> None:
    note = injected_files_system_note(["README.md", "./src/main.rs"])
    assert "readme.md" in note.lower()
    assert "src/main.rs" in note.lower()
    assert "fs.read" in note


def test_init_skips_read_when_readme_in_manifest() -> None:
    mock_client = MagicMock()
    mock_client.list_dir.return_value = (True, "README.md\nCargo.toml")

    state = _ask_state(injected_files=["README.md"])
    with patch(
        "rex_agent.graph.nodes.init.deterministic_init_enabled", return_value=True
    ):
        result = init_node.init_workspace_node(state, client=mock_client)

    mock_client.read_file.assert_not_called()
    mock_client.list_dir.assert_called_once()
    assert result.get("workspace_explored") is True
    assert result.get("tool_steps") == 1


def test_init_reads_readme_when_not_in_manifest() -> None:
    mock_client = MagicMock()
    mock_client.read_file.return_value = (True, "# rex\n")
    mock_client.list_dir.return_value = (True, "README.md")

    state = _ask_state(injected_files=[])
    with patch(
        "rex_agent.graph.nodes.init.deterministic_init_enabled", return_value=True
    ):
        init_node.init_workspace_node(state, client=mock_client)

    mock_client.read_file.assert_called_once_with("README.md", "ask")
    mock_client.list_dir.assert_called_once()


def test_path_in_injected_manifest_normalizes() -> None:
    assert init_node.path_in_injected_manifest("README.md", ["./README.md"])
    assert not init_node.path_in_injected_manifest("Cargo.toml", ["README.md"])

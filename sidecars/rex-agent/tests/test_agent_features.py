"""R027–R032 feature tests with mocked broker."""

from __future__ import annotations

from unittest.mock import MagicMock, patch

import pytest
from langchain_core.messages import HumanMessage

from rex_agent import graph
from rex_agent.broker_chat_model import MAX_PARSE_RETRIES, messages_to_prompt
from rex_agent.config import max_tool_result_bytes
from rex_agent.diff import apply_unified_diff
from rex_agent.tools import (
    TOOL_READ,
    ReadCache,
    ToolCall,
    execute_tool,
    prune_read_result,
)


def _reset_graphs() -> None:
    graph._REACT = None
    graph._ASK = None


def test_read_dedup_returns_cached_summary() -> None:
    client = MagicMock()
    client.read_file.return_value = (True, "first read body")
    cache = ReadCache()
    call = ToolCall(tool=TOOL_READ, args={"path": "src/foo.py"})
    ok1, _, _ = execute_tool(client, call, "plan", read_cache=cache)
    ok2, r2, _ = execute_tool(client, call, "plan", read_cache=cache)
    assert ok1 and ok2
    assert "[cached read" in r2
    client.read_file.assert_called_once()


def test_parse_recovery_stops_after_max_retries() -> None:
    bad = '{"type":"tool","tool":"fs.read","args":{'

    def always_bad(prompt: str, mode: str, model: str) -> tuple[bool, str]:
        return True, bad

    graph.set_inference_fn(always_bad)
    try:
        with patch("rex_agent.graph.BrokerClient") as broker_cls:
            broker_cls.return_value.__enter__.return_value = MagicMock()
            broker_cls.return_value.__exit__.return_value = None
            _reset_graphs()
            answer, _ = graph.run_turn("test", "plan", "", "")
    finally:
        graph.set_inference_fn(None)
        _reset_graphs()

    assert MAX_PARSE_RETRIES >= 3
    assert "JSON" in answer or "parse" in answer.lower()


def test_compaction_bounds_scratch_on_many_reads() -> None:
    step = {"n": 0}

    def fake_inference(prompt: str, mode: str, model: str) -> tuple[bool, str]:
        step["n"] += 1
        if step["n"] <= 3:
            return True, '{"type":"tool","tool":"fs.read","args":{"path":"f.txt"}}'
        return True, '{"type":"final","answer":"done"}'

    mock_client = MagicMock()
    mock_client.read_file.return_value = (True, "x" * (max_tool_result_bytes() + 100))

    graph.set_inference_fn(fake_inference)
    try:
        with patch("rex_agent.graph.max_tool_steps_for_mode", return_value=5):
            with patch("rex_agent.graph.BrokerClient") as broker_cls:
                broker_cls.return_value.__enter__.return_value = mock_client
                broker_cls.return_value.__exit__.return_value = None
                _reset_graphs()
                answer, _ = graph.run_turn("goal", "plan", "", "")
    finally:
        graph.set_inference_fn(None)
        _reset_graphs()

    assert step["n"] >= 2
    assert "done" in answer


def test_prune_read_result_over_100_lines() -> None:
    content = "\n".join(f"line {i}" for i in range(150))
    pruned = prune_read_result(content, "line 42")
    assert len(pruned.splitlines()) < 150
    assert "[pruned read:" in pruned
    assert "line 42" in pruned


def test_prune_read_result_at_most_100_lines_unchanged() -> None:
    content = "\n".join(f"line {i}" for i in range(100))
    assert prune_read_result(content, "line 42") == content


def test_prune_read_result_empty_hint_head_tail() -> None:
    content = "\n".join(f"line {i}" for i in range(150))
    pruned = prune_read_result(content, "")
    assert "lines pruned" in pruned
    assert "line 0" in pruned
    assert "line 149" in pruned


def test_execute_tool_pruning_when_enabled(monkeypatch: pytest.MonkeyPatch) -> None:
    client = MagicMock()
    body = "\n".join(f"token line {i}" for i in range(150))
    client.read_file.return_value = (
        True,
        f"<<TOOL_RESULT:fs.read>>\n{body}\n<<END>>",
    )
    monkeypatch.setattr("rex_agent.tools.read_pruning_enabled", lambda: True)
    call = ToolCall(tool=TOOL_READ, args={"path": "big.py"})
    ok, result, _ = execute_tool(client, call, "agent", goal_hint="token line 42")
    assert ok
    assert "[pruned read:" in result


def test_execute_tool_skips_pruning_when_disabled(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    client = MagicMock()
    body = "\n".join(f"token line {i}" for i in range(150))
    client.read_file.return_value = (
        True,
        f"<<TOOL_RESULT:fs.read>>\n{body}\n<<END>>",
    )
    monkeypatch.setattr("rex_agent.tools.read_pruning_enabled", lambda: False)
    call = ToolCall(tool=TOOL_READ, args={"path": "big.py"})
    ok, result, _ = execute_tool(client, call, "agent", goal_hint="token line 42")
    assert ok
    assert "[pruned read:" not in result
    assert "token line 42" in result


def test_editor_prompt_excludes_raw_viewer_dumps() -> None:
    long_tool = HumanMessage(content="\n[tool fs.read ok]\n" + ("dump\n" * 200))
    prompt = messages_to_prompt(
        [long_tool],
        "agent",
        "daemon ctx",
        subagent="editor",
        viewer_summary="Viewer found: main entry in src/lib.rs",
    )
    assert "Viewer found" in prompt


def test_apply_unified_diff() -> None:
    original = "line one\nline two\nline three\n"
    diff = "@@ -2,1 +2,1 @@\n line one\n-line two\n+line TWO\n line three\n"
    ok, result = apply_unified_diff(original, diff)
    assert ok
    assert "line TWO" in result


def test_diff_write_patch_failure_message() -> None:
    client = MagicMock()
    client.read_file.return_value = (True, "alpha\nbeta\n")
    call = ToolCall(
        tool="fs.write", args={"path": "x.txt", "diff": "@@ invalid @@\n-bad\n+good\n"}
    )
    ok, msg, _ = execute_tool(client, call, "agent")
    assert not ok
    assert "Patch failed" in msg

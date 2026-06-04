"""Graph loop tests with mocked inference."""

from __future__ import annotations

from unittest.mock import MagicMock, patch

from rex_agent import graph
from rex_agent.tools import TOOL_READ


def _reset_graphs() -> None:
    graph._REACT = None
    graph._ASK = None


def test_ask_mode_single_inference_no_tools() -> None:
    calls: list[str] = []

    def fake_inference(prompt: str, mode: str, model: str) -> tuple[bool, str]:
        calls.append(mode)
        return True, "hello stub"

    graph.set_inference_fn(fake_inference)
    try:
        answer, parts = graph.run_turn("hello mvp", "ask", "", "turn-1")
    finally:
        graph.set_inference_fn(None)
        _reset_graphs()
    assert answer == "hello stub"
    assert calls == ["ask"]


def test_plan_mode_executes_one_tool() -> None:
    step = {"n": 0}

    def fake_inference(prompt: str, mode: str, model: str) -> tuple[bool, str]:
        step["n"] += 1
        if step["n"] == 1:
            return True, '{"type":"tool","tool":"fs.read","args":{"path":"README.md"}}'
        return True, '{"type":"final","answer":"done reading"}'

    mock_client = MagicMock()
    mock_client.read_file.return_value = (True, "file contents")

    graph.set_inference_fn(fake_inference)
    try:
        with patch("rex_agent.graph.BrokerClient") as broker_cls:
            broker_cls.return_value.__enter__.return_value = mock_client
            broker_cls.return_value.__exit__.return_value = None
            _reset_graphs()
            answer, parts = graph.run_turn("read readme", "plan", "", "")
    finally:
        graph.set_inference_fn(None)
        _reset_graphs()

    assert "done reading" in answer
    mock_client.read_file.assert_called_once()
    combined = "".join(parts)
    assert "file contents" in combined or TOOL_READ in combined


def test_tool_step_limit_stops_loop() -> None:
    def always_tool(prompt: str, mode: str, model: str) -> tuple[bool, str]:
        return True, '{"type":"tool","tool":"fs.list","args":{"path":""}}'

    mock_client = MagicMock()
    mock_client.list_dir.return_value = (True, "a.txt")

    graph.set_inference_fn(always_tool)
    try:
        with patch("rex_agent.graph.max_tool_steps", return_value=1):
            with patch("rex_agent.graph.BrokerClient") as broker_cls:
                broker_cls.return_value.__enter__.return_value = mock_client
                broker_cls.return_value.__exit__.return_value = None
                _reset_graphs()
                answer, _ = graph.run_turn("list forever", "plan", "", "")
    finally:
        graph.set_inference_fn(None)
        _reset_graphs()

    assert "agent.max_tool_steps" in answer

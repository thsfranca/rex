"""Graph loop tests with mocked inference."""

from __future__ import annotations

from unittest.mock import MagicMock, patch

from rex_agent import graph
from rex_agent.broker import BrokerToolCall, InferenceResult, legacy_inference_result
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
        with patch("rex_agent.graph.max_tool_steps_for_mode", return_value=1):
            with patch("rex_agent.graph.BrokerClient") as broker_cls:
                broker_cls.return_value.__enter__.return_value = mock_client
                broker_cls.return_value.__exit__.return_value = None
                _reset_graphs()
                answer, _ = graph.run_turn("list forever", "plan", "", "")
    finally:
        graph.set_inference_fn(None)
        _reset_graphs()

    assert "agent.max_tool_steps_plan" in answer


def test_plan_mode_batches_three_reads_one_step() -> None:
    step = {"n": 0}

    def fake_inference(
        prompt: str, mode: str, model: str, **kwargs: object
    ) -> InferenceResult:
        tools = kwargs.get("tools")
        step["n"] += 1
        if step["n"] == 1 and tools:
            return InferenceResult(
                ok=True,
                tool_calls=[
                    BrokerToolCall(tool="fs.read", args={"path": "a.md"}),
                    BrokerToolCall(tool="fs.read", args={"path": "b.md"}),
                    BrokerToolCall(tool="fs.read", args={"path": "c.md"}),
                ],
            )
        return legacy_inference_result(
            True, '{"type":"final","answer":"batched reads done"}'
        )

    mock_client = MagicMock()
    mock_client.read_file.return_value = (True, "content")

    graph.set_inference_fn(fake_inference)
    try:
        with patch("rex_agent.graph.BrokerClient") as broker_cls:
            broker_cls.return_value.__enter__.return_value = mock_client
            broker_cls.return_value.__exit__.return_value = None
            _reset_graphs()
            answer, _ = graph.run_turn("read three", "plan", "", "")
    finally:
        graph.set_inference_fn(None)
        _reset_graphs()

    assert "batched reads done" in answer
    assert mock_client.read_file.call_count == 3


def test_plan_mode_native_tool_call_routes_to_tools() -> None:
    step = {"n": 0}

    def fake_inference(
        prompt: str, mode: str, model: str, **kwargs: object
    ) -> InferenceResult:
        tools = kwargs.get("tools")
        step["n"] += 1
        if step["n"] == 1 and tools:
            return InferenceResult(
                ok=True,
                tool_calls=[
                    BrokerToolCall(tool="fs.read", args={"path": "README.md"})
                ],
            )
        return legacy_inference_result(
            True, '{"type":"final","answer":"done reading native"}'
        )

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

    assert "done reading native" in answer
    mock_client.read_file.assert_called_once()


def test_interim_fallback_retries_without_tools() -> None:
    attempts: list[bool] = []

    def fake_inference(
        prompt: str, mode: str, model: str, **kwargs: object
    ) -> InferenceResult:
        tools = kwargs.get("tools") or []
        has_tools = bool(tools)
        attempts.append(has_tools)
        if has_tools:
            return InferenceResult(
                ok=False,
                error="native_tools_unsupported",
                protocol=3,
            )
        if len(attempts) == 2:
            return legacy_inference_result(
                True,
                '{"type":"tool","tool":"fs.read","args":{"path":"README.md"}}',
            )
        return legacy_inference_result(
            True, '{"type":"final","answer":"done after fallback"}'
        )

    mock_client = MagicMock()
    mock_client.read_file.return_value = (True, "file contents")

    graph.set_inference_fn(fake_inference)
    try:
        with patch("rex_agent.graph.BrokerClient") as broker_cls:
            broker_cls.return_value.__enter__.return_value = mock_client
            broker_cls.return_value.__exit__.return_value = None
            _reset_graphs()
            answer, _ = graph.run_turn("read readme", "plan", "", "")
    finally:
        graph.set_inference_fn(None)
        _reset_graphs()

    assert attempts[0] is True
    assert attempts[1] is False
    assert "done after fallback" in answer
    mock_client.read_file.assert_called_once()


def test_agent_denied_exec_does_not_bill_step() -> None:
    step = {"n": 0}

    def fake_inference(
        prompt: str, mode: str, model: str, **kwargs: object
    ) -> InferenceResult:
        tools = kwargs.get("tools") or []
        step["n"] += 1
        if step["n"] == 1 and tools:
            return InferenceResult(
                ok=True,
                tool_calls=[
                    BrokerToolCall(
                        tool="fs.write",
                        args={"path": ".env", "content": "secret"},
                    )
                ],
            )
        return legacy_inference_result(
            True, '{"type":"final","answer":"done after denied write"}'
        )

    mock_client = MagicMock()
    mock_client.read_file.return_value = (False, "missing")
    mock_client.write_file.return_value = (
        False,
        "access policy denied (protected_path): read denied for .env",
    )

    graph.set_inference_fn(fake_inference)
    try:
        with patch("rex_agent.graph.max_tool_steps_for_mode", return_value=1):
            with patch("rex_agent.graph.BrokerClient") as broker_cls:
                broker_cls.return_value.__enter__.return_value = mock_client
                broker_cls.return_value.__exit__.return_value = None
                _reset_graphs()
                answer, _ = graph.run_turn("write env", "agent", "", "")
    finally:
        graph.set_inference_fn(None)
        _reset_graphs()

    assert "done after denied write" in answer
    assert "agent.max_tool_steps" not in answer
    mock_client.write_file.assert_called_once()

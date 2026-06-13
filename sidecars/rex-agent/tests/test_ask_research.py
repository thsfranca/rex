"""Ask-mode workspace research synthesis tests."""

from __future__ import annotations

from unittest.mock import MagicMock, patch

from langchain_core.messages import HumanMessage

from rex_agent import graph
from rex_agent.broker import BrokerToolCall, InferenceResult, legacy_inference_result
from rex_agent.graph.compaction import compact_state
from rex_agent.tools import TOOL_READ, TOOL_WEB_SEARCH

MARKER = "REX_MARKER_LOCAL_RUNTIME"
README_BODY = (
    f"<<TOOL_RESULT:fs.read>>\n# REX\n\nREX is a local AI runtime. {MARKER}\n<<END>>"
)


def _reset_graphs() -> None:
    graph._REACT = None
    graph._ASK = None


def test_compact_state_skips_ask_mode_when_under_threshold() -> None:
    state = {
        "mode": "ask",
        "messages": [
            HumanMessage(
                content="\n[tool fs.read ok]\nshort",
                id="msg-1",
            )
        ],
        "active_subagent": "viewer",
        "tool_steps": 1,
        "turn_id": "turn-1",
    }
    assert compact_state(state) == {}


def test_compact_state_runs_for_ask_when_over_threshold() -> None:
    state = {
        "mode": "ask",
        "messages": [
            HumanMessage(
                content=f"\n[tool fs.read ok]\n{'x' * 5000}",
                id="msg-1",
            )
        ],
        "active_subagent": "viewer",
        "tool_steps": 1,
        "turn_id": "turn-1",
    }
    result = compact_state(state)
    assert "viewer_summary" in result


def test_ask_research_read_then_final_answer() -> None:
    step = {"n": 0}

    def fake_inference(
        prompt: str, mode: str, model: str, **kwargs: object
    ) -> InferenceResult:
        tools = kwargs.get("tools") or []
        step["n"] += 1
        if step["n"] == 1 and tools:
            return InferenceResult(
                ok=True,
                tool_calls=[BrokerToolCall(tool="fs.read", args={"path": "README.md"})],
            )
        return legacy_inference_result(
            True,
            f'{{"type":"final","answer":"REX is described in README: {MARKER}"}}',
        )

    mock_client = MagicMock()
    mock_client.read_file.return_value = (True, README_BODY)

    graph.set_inference_fn(fake_inference)
    try:
        with patch("rex_agent.graph.BrokerClient") as broker_cls:
            broker_cls.return_value.__enter__.return_value = mock_client
            broker_cls.return_value.__exit__.return_value = None
            _reset_graphs()
            answer, parts = graph.run_turn(
                "Can you tell me what rex is?", "ask", "", "turn-ask"
            )
    finally:
        graph.set_inference_fn(None)
        _reset_graphs()

    assert MARKER in answer
    assert step["n"] <= 3
    mock_client.read_file.assert_called_once()
    combined = "".join(parts)
    assert MARKER in combined or TOOL_READ in combined


def test_ask_step_limit_returns_terminal_message() -> None:
    step = {"n": 0}

    def fake_inference(
        prompt: str, mode: str, model: str, **kwargs: object
    ) -> InferenceResult:
        tools = kwargs.get("tools") or []
        step["n"] += 1
        if tools:
            return InferenceResult(
                ok=True,
                tool_calls=[BrokerToolCall(tool="fs.list", args={"path": ""})],
            )
        return legacy_inference_result(
            True,
            f'{{"type":"final","answer":"should not reach"}}',
        )

    mock_client = MagicMock()
    mock_client.list_dir.return_value = (True, "README.md")

    graph.set_inference_fn(fake_inference)
    try:
        with patch("rex_agent.graph.max_tool_steps_for_mode", return_value=1):
            with patch("rex_agent.graph.BrokerClient") as broker_cls:
                broker_cls.return_value.__enter__.return_value = mock_client
                broker_cls.return_value.__exit__.return_value = None
                _reset_graphs()
                answer, _ = graph.run_turn("list forever", "ask", "", "")
    finally:
        graph.set_inference_fn(None)
        _reset_graphs()

    assert "agent.max_tool_steps_ask" in answer


def test_ask_mode_batches_two_reads_one_step() -> None:
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
                ],
            )
        return legacy_inference_result(True, '{"type":"final","answer":"ask batch ok"}')

    mock_client = MagicMock()
    mock_client.read_file.return_value = (True, "body")

    graph.set_inference_fn(fake_inference)
    try:
        with patch("rex_agent.graph.BrokerClient") as broker_cls:
            broker_cls.return_value.__enter__.return_value = mock_client
            broker_cls.return_value.__exit__.return_value = None
            _reset_graphs()
            answer, _ = graph.run_turn("research", "ask", "", "")
    finally:
        graph.set_inference_fn(None)
        _reset_graphs()

    assert "ask batch ok" in answer
    assert mock_client.read_file.call_count == 2


def test_ask_cap_regression_denied_web_search_does_not_exhaust_cap() -> None:
    """Reproduce policy-denied web.search loops without hitting step cap."""
    step = {"n": 0}

    def fake_inference(
        prompt: str, mode: str, model: str, **kwargs: object
    ) -> InferenceResult:
        tools = kwargs.get("tools") or []
        step["n"] += 1
        if step["n"] <= 2 and tools:
            return InferenceResult(
                ok=True,
                tool_calls=[
                    BrokerToolCall(
                        tool="web.search",
                        args={"query": "rex programming language"},
                    )
                ],
            )
        if step["n"] == 3 and tools:
            return InferenceResult(
                ok=True,
                tool_calls=[BrokerToolCall(tool="fs.read", args={"path": "README.md"})],
            )
        return legacy_inference_result(
            True,
            f'{{"type":"final","answer":"REX is described in README: {MARKER}"}}',
        )

    mock_client = MagicMock()
    mock_client.web_search.return_value = (
        False,
        "access policy denied (mode_denied): web.search denied for mode ask",
    )
    mock_client.read_file.return_value = (True, README_BODY)

    graph.set_inference_fn(fake_inference)
    try:
        with patch("rex_agent.config.search_enabled", return_value=True):
            with patch("rex_agent.graph.max_tool_steps_for_mode", return_value=5):
                with patch("rex_agent.graph.BrokerClient") as broker_cls:
                    broker_cls.return_value.__enter__.return_value = mock_client
                    broker_cls.return_value.__exit__.return_value = None
                    _reset_graphs()
                    answer, _ = graph.run_turn(
                        "search the web for what rex is", "ask", "", "cap-regression"
                    )
    finally:
        graph.set_inference_fn(None)
        _reset_graphs()

    assert MARKER in answer
    assert "agent.max_tool_steps_ask" not in answer
    assert mock_client.web_search.call_count == 2
    mock_client.read_file.assert_called_once()

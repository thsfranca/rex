"""Golden prompt regression suite (R064)."""

from __future__ import annotations

from unittest.mock import MagicMock, patch

from rex_agent import graph
from rex_agent.broker import BrokerToolCall, InferenceResult
from rex_agent.metrics import CAP_TERMINAL_COMPLETE, classify_loop_terminal

MARKER = "REX_GOLDEN_MARKER"
README = f"# Rex\n\nLocal runtime marker {MARKER}"


def _reset_graphs() -> None:
    graph._REACT = None
    graph._ASK = None


def test_golden_ask_what_is_rex_completes_without_cap_terminal() -> None:
    llm_calls = {"n": 0}

    def fake_inference(
        prompt: str, mode: str, model: str, **kwargs: object
    ) -> InferenceResult:
        llm_calls["n"] += 1
        tools = kwargs.get("tools") or []
        if llm_calls["n"] == 1 and tools:
            return InferenceResult(
                ok=True,
                tool_calls=[],
                text='{"type":"final","answer":"Rex is a local AI runtime."}',
            )
        return InferenceResult(ok=True, text='{"type":"final","answer":"fallback"}')

    mock_client = MagicMock()
    mock_client.read_file.return_value = (True, README)
    mock_client.list_dir.return_value = (True, "README.md\nCargo.toml")

    graph.set_inference_fn(fake_inference)
    try:
        with patch("rex_agent.graph.nodes.init.deterministic_init_enabled", return_value=True):
            with patch("rex_agent.graph.BrokerClient") as broker_cls:
                broker_cls.return_value.__enter__.return_value = mock_client
                broker_cls.return_value.__exit__.return_value = None
                _reset_graphs()
                answer, _ = graph.run_turn("What is rex?", "ask", "", "golden-ask")
    finally:
        graph.set_inference_fn(None)
        _reset_graphs()

    assert llm_calls["n"] == 1
    assert "runtime" in answer.lower() or "rex" in answer.lower()


def test_classify_loop_terminal_complete() -> None:
    assert (
        classify_loop_terminal({"done": True, "final_answer": "done", "tool_steps": 2})
        == CAP_TERMINAL_COMPLETE
    )

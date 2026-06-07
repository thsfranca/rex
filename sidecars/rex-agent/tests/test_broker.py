"""Broker client unit tests (mocked gRPC stub)."""

from __future__ import annotations

from unittest.mock import MagicMock, patch

import grpc

from rex_agent.broker import (
    BrokerClient,
    InferenceResult,
    is_interim_fallback,
    legacy_inference_result,
    strip_tool_result_delimiters,
    truncate_tool_result,
)
from rex_agent.config import DEFAULT_MAX_TOOL_RESULT_BYTES


class _ReadOk:
    ok = True
    content = "<<TOOL_RESULT:fs.read>>\nfile-body\n<<END>>"
    error = ""


class _ListOk:
    ok = True
    error = ""

    class _Entry:
        def __init__(self, name: str, is_dir: bool) -> None:
            self.name = name
            self.is_dir = is_dir

    entries = [_Entry("src", True), _Entry("main.rs", False)]


def test_strip_tool_result_delimiters() -> None:
    payload = "<<TOOL_RESULT:fs.read>>\nline-one\nline-two\n<<END>>"
    assert strip_tool_result_delimiters(payload) == "line-one\nline-two"
    assert strip_tool_result_delimiters("plain text") == "plain text"


def test_truncate_tool_result_adds_ellipsis() -> None:
    text = "x" * (DEFAULT_MAX_TOOL_RESULT_BYTES + 64)
    result = truncate_tool_result(text)
    assert len(result.encode("utf-8")) <= DEFAULT_MAX_TOOL_RESULT_BYTES + 4
    assert result.endswith("\u2026")


def test_broker_inference_sends_turn_metadata() -> None:
    stub = MagicMock()
    response = MagicMock()
    response.ok = True
    response.text = "answer"
    response.content = "answer"
    response.error = ""
    response.tool_calls = []
    response.protocol = 2
    stub.BrokerInference.return_value = response

    with patch("rex_agent.broker._daemon_channel", return_value=MagicMock()):
        with BrokerClient(turn_id="turn-1") as client:
            client._stub = stub
            result = client.inference("hi", "ask", "model-x")
    assert result.ok is True
    assert result.effective_text() == "answer"
    _request, kwargs = stub.BrokerInference.call_args
    assert kwargs["metadata"] == (("x-rex-turn-id", "turn-1"),)


def test_read_file_passes_through_delimited_content() -> None:
    stub = MagicMock()
    stub.BrokerReadFile.return_value = _ReadOk()

    with patch("rex_agent.broker._daemon_channel", return_value=MagicMock()):
        with BrokerClient() as client:
            client._stub = stub
            ok, text = client.read_file("src/main.rs", "agent")
    assert ok is True
    assert text.startswith("<<TOOL_RESULT:fs.read>>")
    assert "file-body" in text
    assert text.endswith("<<END>>")


def test_read_file_denied_returns_error_string() -> None:
    stub = MagicMock()
    response = MagicMock()
    response.ok = False
    response.error = "access denied"
    stub.BrokerReadFile.return_value = response

    with patch("rex_agent.broker._daemon_channel", return_value=MagicMock()):
        with BrokerClient() as client:
            client._stub = stub
            ok, msg = client.read_file("secret.env", "plan")
    assert ok is False
    assert "access denied" in msg


def test_list_dir_formats_entries() -> None:
    stub = MagicMock()
    stub.BrokerListDir.return_value = _ListOk()

    with patch("rex_agent.broker._daemon_channel", return_value=MagicMock()):
        with BrokerClient() as client:
            client._stub = stub
            ok, text = client.list_dir("", "plan")
    assert ok is True
    assert "src/" in text
    assert "main.rs" in text


def test_save_plan_calls_broker_rpc() -> None:
    stub = MagicMock()
    response = MagicMock()
    response.ok = True
    response.error = ""
    stub.BrokerSavePlan.return_value = response

    with patch("rex_agent.broker._daemon_channel", return_value=MagicMock()):
        with BrokerClient() as client:
            client._stub = stub
            ok, msg = client.save_plan(
                ".rex/plans/feature.md", "# Plan\n", "plan"
            )
    assert ok is True
    assert msg == "ok"
    stub.BrokerSavePlan.assert_called_once()


def test_grpc_error_surfaces_as_failure() -> None:
    stub = MagicMock()
    stub.BrokerInference.side_effect = grpc.RpcError("unavailable")

    with patch("rex_agent.broker._daemon_channel", return_value=MagicMock()):
        with BrokerClient() as client:
            client._stub = stub
            result = client.inference("x", "ask", "")
    assert result.ok is False
    assert result.error


def test_broker_inference_sends_messages_and_tools() -> None:
    from rex_agent.broker import rex_pb2

    stub = MagicMock()
    response = MagicMock()
    response.ok = True
    response.text = ""
    response.content = ""
    response.error = ""
    response.tool_calls = []
    response.protocol = 1
    stub.BrokerInference.return_value = response

    messages = [rex_pb2.ChatMessage(role="user", content="hello")]
    tools = [
        rex_pb2.ToolSpec(
            name="fs.read",
            description="read",
            parameters_json='{"type":"object"}',
        )
    ]

    with patch("rex_agent.broker._daemon_channel", return_value=MagicMock()):
        with BrokerClient() as client:
            client._stub = stub
            client.inference(
                "prompt",
                "plan",
                "m",
                messages=messages,
                tools=tools,
            )

    request = stub.BrokerInference.call_args[0][0]
    assert len(request.messages) == 1
    assert request.messages[0].content == "hello"
    assert len(request.tools) == 1
    assert request.tools[0].name == "fs.read"


def test_broker_inference_maps_native_tool_calls() -> None:
    stub = MagicMock()

    class _ToolCall:
        name = "fs.read"
        arguments_json = '{"path":"README.md"}'

    response = MagicMock()
    response.ok = True
    response.text = ""
    response.content = ""
    response.error = ""
    response.tool_calls = [_ToolCall()]
    response.protocol = 1
    stub.BrokerInference.return_value = response

    with patch("rex_agent.broker._daemon_channel", return_value=MagicMock()):
        with BrokerClient() as client:
            client._stub = stub
            result = client.inference("x", "plan", "")

    assert result.ok is True
    assert len(result.tool_calls) == 1
    assert result.tool_calls[0].tool == "fs.read"
    assert result.tool_calls[0].args == {"path": "README.md"}


def test_is_interim_fallback() -> None:
    from rex_agent.broker import rex_pb2

    fallback = InferenceResult(
        ok=False,
        error="native_tools_unsupported: model lacks tools",
        protocol=rex_pb2.INFERENCE_PROTOCOL_INTERIM_FALLBACK,
    )
    assert is_interim_fallback(fallback) is True
    assert is_interim_fallback(legacy_inference_result(False, "other")) is False

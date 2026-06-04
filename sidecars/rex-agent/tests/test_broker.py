"""Broker client unit tests (mocked gRPC stub)."""

from __future__ import annotations

from unittest.mock import MagicMock, patch

import grpc

from rex_agent.broker import BrokerClient, truncate_tool_result
from rex_agent.config import DEFAULT_MAX_TOOL_RESULT_BYTES


class _ReadOk:
    ok = True
    content = "file-body"
    error = ""


class _ListOk:
    ok = True
    error = ""

    class _Entry:
        def __init__(self, name: str, is_dir: bool) -> None:
            self.name = name
            self.is_dir = is_dir

    entries = [_Entry("src", True), _Entry("main.rs", False)]


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
    response.error = ""
    stub.BrokerInference.return_value = response

    with patch("rex_agent.broker._daemon_channel", return_value=MagicMock()):
        with BrokerClient(turn_id="turn-1") as client:
            client._stub = stub
            ok, text = client.inference("hi", "ask", "model-x")
    assert ok is True
    assert text == "answer"
    _request, kwargs = stub.BrokerInference.call_args
    assert kwargs["metadata"] == (("x-rex-turn-id", "turn-1"),)


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


def test_grpc_error_surfaces_as_failure() -> None:
    stub = MagicMock()
    stub.BrokerInference.side_effect = grpc.RpcError("unavailable")

    with patch("rex_agent.broker._daemon_channel", return_value=MagicMock()):
        with BrokerClient() as client:
            client._stub = stub
            ok, msg = client.inference("x", "ask", "")
    assert ok is False
    assert msg

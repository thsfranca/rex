"""Broker client: daemon UDS gRPC (rex.v1)."""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import TYPE_CHECKING

import grpc

from rex_agent.config import daemon_socket, max_tool_result_bytes

if TYPE_CHECKING:
    from rex.v1 import rex_pb2, rex_pb2_grpc

try:
    from rex.v1 import rex_pb2, rex_pb2_grpc
except ImportError as exc:  # pragma: no cover
    raise ImportError(
        "rex.v1 protobuf stubs not found. Run `rex proto install` and set "
        "PYTHONPATH to $(rex proto path)."
    ) from exc

BROKER_TIMEOUT_SEC = 30.0
_TOOL_RESULT_PATTERN = re.compile(
    r"^<<TOOL_RESULT:(?P<tool>[^>]+)>>\n(?P<body>.*)\n<<END>>\s*$",
    re.DOTALL,
)


def _daemon_channel(socket_path: str) -> grpc.Channel:
    target = (
        socket_path
        if socket_path.startswith("unix://")
        else f"unix://{socket_path}"
    )
    options = (
        ("grpc.default_authority", "localhost"),
        ("grpc.enable_http_proxy", 0),
        ("grpc.http2.max_pings_without_data", 0),
        ("grpc.keepalive_permit_without_calls", 1),
    )
    return grpc.insecure_channel(target, options=options)


def _metadata(turn_id: str | None) -> tuple[tuple[str, str], ...]:
    if turn_id and turn_id.strip():
        return (("x-rex-turn-id", turn_id.strip()),)
    return ()


@dataclass
class ShellResult:
    stdout: str
    stderr: str


class BrokerClient:
    """One gRPC channel per turn; closes on exit."""

    def __init__(self, turn_id: str | None = None) -> None:
        self._socket = daemon_socket()
        self._channel = _daemon_channel(self._socket)
        self._stub = rex_pb2_grpc.RexServiceStub(self._channel)
        self._turn_id = turn_id
        self._mode = "ask"

    def close(self) -> None:
        self._channel.close()

    def __enter__(self) -> BrokerClient:
        return self

    def __exit__(self, *args: object) -> None:
        self.close()

    def inference(self, prompt: str, mode: str, model: str) -> tuple[bool, str]:
        self._mode = mode or "ask"
        request = rex_pb2.BrokerInferenceRequest(
            prompt=prompt,
            mode=self._mode,
            model=model or "",
        )
        try:
            response = self._stub.BrokerInference(
                request,
                timeout=BROKER_TIMEOUT_SEC,
                metadata=_metadata(self._turn_id),
            )
        except grpc.RpcError as err:
            return False, str(err)
        if response.ok:
            return True, response.text
        return False, response.error or "broker inference failed"

    def read_file(self, path: str, mode: str | None = None) -> tuple[bool, str]:
        request = rex_pb2.BrokerReadFileRequest(
            path=path,
            mode=mode or self._mode,
        )
        return self._read_file_response(request)

    def list_dir(self, path: str, mode: str | None = None) -> tuple[bool, str]:
        request = rex_pb2.BrokerListDirRequest(
            path=path,
            mode=mode or self._mode,
        )
        try:
            response = self._stub.BrokerListDir(
                request,
                timeout=BROKER_TIMEOUT_SEC,
                metadata=_metadata(self._turn_id),
            )
        except grpc.RpcError as err:
            return False, str(err)
        if not response.ok:
            return False, response.error or "broker list_dir failed"
        lines = []
        for entry in response.entries:
            name = entry.name
            lines.append(f"{name}/" if entry.is_dir else name)
        text = ", ".join(lines) if lines else "(empty)"
        return True, truncate_tool_result(text)

    def write_file(
        self, path: str, content: str, mode: str | None = None
    ) -> tuple[bool, str]:
        request = rex_pb2.BrokerWriteFileRequest(
            path=path,
            content=content,
            mode=mode or self._mode,
        )
        try:
            response = self._stub.BrokerWriteFile(
                request,
                timeout=BROKER_TIMEOUT_SEC,
                metadata=_metadata(self._turn_id),
            )
        except grpc.RpcError as err:
            return False, str(err)
        if response.ok:
            return True, "ok"
        return False, response.error or "broker write_file failed"

    def exec_shell(self, command: str, mode: str | None = None) -> tuple[bool, str]:
        request = rex_pb2.BrokerExecShellRequest(
            command=command,
            mode=mode or self._mode,
        )
        try:
            response = self._stub.BrokerExecShell(
                request,
                timeout=BROKER_TIMEOUT_SEC,
                metadata=_metadata(self._turn_id),
            )
        except grpc.RpcError as err:
            return False, str(err)
        if not response.ok:
            return False, response.error or "broker exec_shell failed"
        return True, response.stdout

    def _read_file_response(
        self, request: rex_pb2.BrokerReadFileRequest
    ) -> tuple[bool, str]:
        try:
            response = self._stub.BrokerReadFile(
                request,
                timeout=BROKER_TIMEOUT_SEC,
                metadata=_metadata(self._turn_id),
            )
        except grpc.RpcError as err:
            return False, str(err)
        if response.ok:
            return True, response.content
        return False, response.error or "broker read_file failed"


def broker_inference(
    prompt: str,
    mode: str,
    model: str,
    turn_id: str | None = None,
) -> tuple[bool, str]:
    """Call BrokerInference on the daemon (one-shot; opens a channel per call)."""
    with BrokerClient(turn_id=turn_id) as client:
        return client.inference(prompt, mode, model)


def strip_tool_result_delimiters(text: str) -> str:
    """Return raw tool body from daemon-delimited broker payloads (R034)."""
    match = _TOOL_RESULT_PATTERN.match(text.strip())
    if match:
        return match.group("body")
    return text


def truncate_tool_result(text: str) -> str:
    """Align sidecar scratch with broker.max_tool_result_bytes (fs.list only)."""
    limit = max_tool_result_bytes()
    encoded = text.encode("utf-8")
    if len(encoded) <= limit:
        return text
    trimmed = encoded[:limit].decode("utf-8", errors="ignore")
    return trimmed + "…"

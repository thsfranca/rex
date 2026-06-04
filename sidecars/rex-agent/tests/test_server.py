"""Servicer tests with broker mocked (no daemon or proto gen required at import)."""

from __future__ import annotations

import sys
import types
from unittest.mock import patch

_sidecar_pb2 = types.ModuleType("rex.sidecar.v1.sidecar_pb2")


class _HealthResponse:
    def __init__(self, healthy: bool, version: str) -> None:
        self.healthy = healthy
        self.version = version


class _GetCapabilitiesResponse:
    def __init__(self, capabilities: list[str]) -> None:
        self.capabilities = capabilities


class _RunTurnChunk:
    def __init__(self, text: str, index: int, done: bool) -> None:
        self.text = text
        self.index = index
        self.done = done


class _RunTurnRequest:
    def __init__(
        self,
        prompt: str = "",
        mode: str = "",
        model: str = "",
    ) -> None:
        self.prompt = prompt
        self.mode = mode
        self.model = model


_sidecar_pb2.HealthResponse = _HealthResponse
_sidecar_pb2.GetCapabilitiesResponse = _GetCapabilitiesResponse
_sidecar_pb2.RunTurnChunk = _RunTurnChunk
_sidecar_pb2_grpc = types.ModuleType("rex.sidecar.v1.sidecar_pb2_grpc")


class _SidecarServiceServicer:
    pass


def _add_sidecar_service_servicer_to_server(servicer, server) -> None:  # noqa: ARG001
    pass


_sidecar_pb2_grpc.SidecarServiceServicer = _SidecarServiceServicer
_sidecar_pb2_grpc.add_SidecarServiceServicer_to_server = _add_sidecar_service_servicer_to_server

_pkg = types.ModuleType("rex.sidecar.v1")
_pkg.sidecar_pb2 = _sidecar_pb2
_pkg.sidecar_pb2_grpc = _sidecar_pb2_grpc
sys.modules["rex.sidecar.v1"] = _pkg
sys.modules["rex.sidecar.v1.sidecar_pb2"] = _sidecar_pb2
sys.modules["rex.sidecar.v1.sidecar_pb2_grpc"] = _sidecar_pb2_grpc

_rex_v1 = types.ModuleType("rex.v1")
_rex_v1.rex_pb2 = types.ModuleType("rex.v1.rex_pb2")
_rex_v1.rex_pb2_grpc = types.ModuleType("rex.v1.rex_pb2_grpc")
sys.modules["rex.v1"] = _rex_v1
sys.modules["rex.v1.rex_pb2"] = _rex_v1.rex_pb2
sys.modules["rex.v1.rex_pb2_grpc"] = _rex_v1.rex_pb2_grpc

_grpc = types.ModuleType("grpc")
_grpc.RpcError = Exception
sys.modules.setdefault("grpc", _grpc)

for mod in ("rex_agent.broker", "rex_agent.server"):
    sys.modules.pop(mod, None)

from rex_agent.server import AgentServicer  # noqa: E402


def test_health_returns_version() -> None:
    servicer = AgentServicer()
    response = servicer.Health(None, None)
    assert response.healthy is True
    assert response.version


def test_run_turn_success_chunks() -> None:
    servicer = AgentServicer()
    request = _RunTurnRequest(prompt="hello", mode="ask", model="")
    with patch("rex_agent.server.stream_turn", return_value=iter(["ok"])):
        chunks = list(servicer.RunTurn(request, None))
    assert chunks[-1].done
    assert "ok" in "".join(c.text for c in chunks if not c.done)


def test_run_turn_inference_failure_message() -> None:
    servicer = AgentServicer()
    request = _RunTurnRequest(prompt="x", mode="agent", model="")
    with patch(
        "rex_agent.server.stream_turn",
        return_value=iter(
            ["Inference failed. Check that the daemon is running and HTTP inference is configured."]
        ),
    ):
        chunks = list(servicer.RunTurn(request, None))
    text = "".join(c.text for c in chunks if not c.done)
    assert "Inference failed" in text

"""rex.capability.v1 gRPC service (Health-only for R056-1)."""

from __future__ import annotations

import os
import signal
import sys
from concurrent import futures
from pathlib import Path

import grpc

from capability_mock import __version__

try:
    from rex.capability.v1 import capability_pb2, capability_pb2_grpc
except ImportError as exc:  # pragma: no cover
    raise ImportError(
        "rex.capability.v1 protobuf stubs not found. Run `rex proto install` and set "
        "PYTHONPATH to $(rex proto path)."
    ) from exc


def sidecar_socket() -> str:
    raw = os.environ.get("REX_SIDECAR_SOCKET", "").strip()
    if raw:
        return raw
    return "/tmp/rex-capability-mock.sock"


def remove_stale_socket(path: str) -> None:
    socket_path = Path(path)
    if socket_path.exists():
        socket_path.unlink()


class MockCapabilityServicer(capability_pb2_grpc.CapabilityServiceServicer):
    def Health(self, request, context):  # noqa: N802, ARG002
        return capability_pb2.HealthResponse(healthy=True, version=__version__)

    def GetCapabilities(self, request, context):  # noqa: N802, ARG002
        return capability_pb2.GetCapabilitiesResponse(
            capability_ids=["web.search"]
        )

    def Invoke(self, request, context):  # noqa: N802, ARG002
        context.set_code(grpc.StatusCode.UNIMPLEMENTED)
        context.set_details("Invoke deferred to R056-3")
        return capability_pb2.InvokeResponse()


def serve(socket_path: str) -> None:
    remove_stale_socket(socket_path)
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=2))
    capability_pb2_grpc.add_CapabilityServiceServicer_to_server(
        MockCapabilityServicer(), server
    )
    server.add_insecure_port(f"unix://{socket_path}")
    server.start()
    sys.stderr.write(
        f"rex-capability-mock event=listen socket={socket_path} version={__version__}\n"
    )
    sys.stderr.flush()

    stopped = False

    def _shutdown(_signum, _frame) -> None:
        nonlocal stopped
        if stopped:
            return
        stopped = True
        server.stop(grace=1)

    signal.signal(signal.SIGINT, _shutdown)
    signal.signal(signal.SIGTERM, _shutdown)
    server.wait_for_termination()
    remove_stale_socket(socket_path)


def main() -> None:
    serve(sidecar_socket())

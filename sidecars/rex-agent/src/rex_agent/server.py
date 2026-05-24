"""rex.sidecar.v1 gRPC service implementation."""

from __future__ import annotations

import time
from collections.abc import Iterator
from typing import TYPE_CHECKING

from rex_agent import __version__
from rex_agent.broker import broker_inference
from rex_agent.streaming import run_turn_chunks

if TYPE_CHECKING:
    import grpc

try:
    from rex.sidecar.v1 import sidecar_pb2, sidecar_pb2_grpc
except ImportError as exc:  # pragma: no cover
    raise ImportError(
        "rex.sidecar.v1 protobuf stubs not found. Run `rex proto install` and set "
        "PYTHONPATH to $(rex proto path)."
    ) from exc

CHUNK_DELAY_SEC = 0.005
RUN_TURN_CAPABILITY = "run_turn"


class AgentServicer(sidecar_pb2_grpc.SidecarServiceServicer):
    """Minimal scaffold: broker inference only, streaming chunks."""

    def Health(self, request, context):  # noqa: N802, ARG002
        return sidecar_pb2.HealthResponse(healthy=True, version=__version__)

    def GetCapabilities(self, request, context):  # noqa: N802, ARG002
        return sidecar_pb2.GetCapabilitiesResponse(capabilities=[RUN_TURN_CAPABILITY])

    def RunTurn(self, request, context) -> Iterator[sidecar_pb2.RunTurnChunk]:  # noqa: N802
        mode = (request.mode or "").strip() or "ask"
        model = request.model or ""
        ok, text = broker_inference(request.prompt, mode, model)
        if not ok:
            text = f"[broker.inference error: {text}]"
        for chunk in run_turn_chunks(text):
            if not chunk.done and CHUNK_DELAY_SEC > 0:
                time.sleep(CHUNK_DELAY_SEC)
            yield sidecar_pb2.RunTurnChunk(
                text=chunk.text,
                index=chunk.index,
                done=chunk.done,
            )


def add_to_server(servicer: AgentServicer, server: grpc.Server) -> None:
    sidecar_pb2_grpc.add_SidecarServiceServicer_to_server(servicer, server)

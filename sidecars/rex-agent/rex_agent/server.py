"""rex.sidecar.v1 gRPC server (async)."""

from __future__ import annotations

import asyncio

import grpc

from rex.sidecar.v1 import sidecar_pb2, sidecar_pb2_grpc

from rex_agent.broker_client import RexBrokerClient
from rex_agent.config import daemon_socket, load_merged_config, sidecar_socket
from rex_agent.graphs.router import run_turn_stream

VERSION = "0.1.0"


class SidecarService(sidecar_pb2_grpc.SidecarServiceServicer):
    def __init__(self) -> None:
        self._cfg = load_merged_config()

    async def Health(self, request, context):  # noqa: ARG002
        return sidecar_pb2.HealthResponse(healthy=True, version=VERSION)

    async def GetCapabilities(self, request, context):  # noqa: ARG002
        return sidecar_pb2.GetCapabilitiesResponse(
            capabilities=[
                "run_turn",
                "tools:fs.read",
                "tools:fs.list",
                "tools:fs.write",
                "tools:exec.shell",
            ]
        )

    async def RunTurn(self, request, context):  # noqa: ARG002
        broker = RexBrokerClient(daemon_socket(self._cfg))
        index = 0
        try:
            async for piece in run_turn_stream(
                broker, request.prompt, request.mode, request.model
            ):
                yield sidecar_pb2.RunTurnChunk(text=piece + "\n", index=index, done=False)
                index += 1
            yield sidecar_pb2.RunTurnChunk(text="", index=index, done=True)
        finally:
            await broker.close()


async def serve_async() -> None:
    cfg = load_merged_config()
    socket_path = sidecar_socket(cfg)
    server = grpc.aio.server()
    sidecar_pb2_grpc.add_SidecarServiceServicer_to_server(SidecarService(), server)
    server.add_insecure_port(f"unix://{socket_path}")
    await server.start()
    print(f"rex-agent listening on {socket_path}")
    await server.wait_for_termination()


def serve() -> None:
    asyncio.run(serve_async())

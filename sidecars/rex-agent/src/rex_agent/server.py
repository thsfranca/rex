"""rex.sidecar.v1 gRPC service implementation."""

from __future__ import annotations

import time
from collections.abc import Iterator
from typing import TYPE_CHECKING

from rex_agent import __version__
from rex_agent.graph import stream_turn
from rex_agent.stream_events import StepStreamEvent, TextStreamEvent, ToolStreamEvent
from rex_agent.streaming import chunk_text

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
    """LangGraph ReAct agent with broker-only LLM and tools."""

    def Health(self, request, context):  # noqa: N802, ARG002
        return sidecar_pb2.HealthResponse(healthy=True, version=__version__)

    def GetCapabilities(self, request, context):  # noqa: N802, ARG002
        return sidecar_pb2.GetCapabilitiesResponse(capabilities=[RUN_TURN_CAPABILITY])

    def RunTurn(self, request, context) -> Iterator[sidecar_pb2.RunTurnChunk]:  # noqa: N802
        mode = (request.mode or "").strip() or "ask"
        model = request.model or ""
        turn_id = getattr(request, "turn_id", "") or ""
        if turn_id:
            print(f"rex-agent event=run_turn turn_id={turn_id} mode={mode}")

        index = 0
        for event in stream_turn(request.prompt, mode, model, turn_id):
            if isinstance(event, TextStreamEvent):
                for piece in chunk_text(event.text):
                    if CHUNK_DELAY_SEC > 0:
                        time.sleep(CHUNK_DELAY_SEC)
                    yield sidecar_pb2.RunTurnChunk(
                        text=piece,
                        index=index,
                        done=False,
                        event="chunk",
                    )
                    index += 1
                continue
            if isinstance(event, ToolStreamEvent):
                yield sidecar_pb2.RunTurnChunk(
                    text="",
                    index=index,
                    done=False,
                    event="tool",
                    tool_name=event.name,
                    phase=event.phase,
                    detail=event.detail,
                )
                index += 1
                continue
            if isinstance(event, StepStreamEvent):
                yield sidecar_pb2.RunTurnChunk(
                    text="",
                    index=index,
                    done=False,
                    event="step",
                    phase=event.phase,
                    summary=event.summary,
                )
                index += 1
        yield sidecar_pb2.RunTurnChunk(
            text="",
            index=index,
            done=True,
        )


def add_to_server(servicer: AgentServicer, server: grpc.Server) -> None:
    sidecar_pb2_grpc.add_SidecarServiceServicer_to_server(servicer, server)

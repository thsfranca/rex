"""rex.sidecar.v1 gRPC service implementation."""

from __future__ import annotations

import time
from collections.abc import Iterator
from typing import TYPE_CHECKING

from rex_agent import __version__
from rex_agent.graph import stream_continue_turn, stream_turn
from rex_agent.stream_events import (
    ActivityStreamEvent,
    PlanStreamEvent,
    StepStreamEvent,
    TextStreamEvent,
    ToolStreamEvent,
)
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
CONTINUE_TURN_CAPABILITY = "continue_turn"


def _chunk_kwargs(
    event: object,
    *,
    index: int,
    turn_id: str,
) -> dict:
    base = {
        "text": "",
        "index": index,
        "done": False,
        "turn_id": turn_id,
    }
    if isinstance(event, TextStreamEvent):
        return {
            **base,
            "text": event.text,
            "event": "chunk",
            "sequence": event.sequence,
        }
    if isinstance(event, ToolStreamEvent):
        kwargs = {
            **base,
            "event": "tool",
            "tool_name": event.name,
            "phase": event.phase,
            "detail": event.detail,
            "tool_call_id": event.tool_call_id,
            "sequence": event.sequence,
        }
        if event.elapsed_ms is not None:
            kwargs["elapsed_ms"] = event.elapsed_ms
        return kwargs
    if isinstance(event, StepStreamEvent):
        return {
            **base,
            "event": "step",
            "phase": event.phase,
            "summary": event.summary,
            "sequence": event.sequence,
        }
    if isinstance(event, ActivityStreamEvent):
        return {
            **base,
            "event": "activity",
            "phase": event.phase,
            "summary": event.summary,
            "detail": event.detail,
            "sequence": event.sequence,
        }
    if isinstance(event, PlanStreamEvent):
        return {
            **base,
            "event": "plan",
            "phase": event.phase,
            "summary": event.title,
            "detail": event.detail,
            "sequence": event.sequence,
        }
    return base


class AgentServicer(sidecar_pb2_grpc.SidecarServiceServicer):
    """LangGraph ReAct agent with broker-only LLM and tools."""

    def Health(self, request, context):  # noqa: N802, ARG002
        return sidecar_pb2.HealthResponse(healthy=True, version=__version__)

    def GetCapabilities(self, request, context):  # noqa: N802, ARG002
        return sidecar_pb2.GetCapabilitiesResponse(
            capabilities=[RUN_TURN_CAPABILITY, CONTINUE_TURN_CAPABILITY]
        )

    def RunTurn(self, request, context) -> Iterator[sidecar_pb2.RunTurnChunk]:  # noqa: N802
        injected = list(getattr(request, "injected_files", None) or [])
        yield from self._stream_events(
            stream_turn(
                request.prompt,
                (request.mode or "").strip() or "ask",
                request.model or "",
                getattr(request, "turn_id", "") or "",
                injected,
            ),
            getattr(request, "turn_id", "") or "",
        )

    def ContinueTurn(self, request, context) -> Iterator[sidecar_pb2.RunTurnChunk]:  # noqa: N802
        turn_id = getattr(request, "turn_id", "") or ""
        token = getattr(request, "continue_token", "") or ""
        yield from self._stream_events(stream_continue_turn(token, turn_id), turn_id)

    def _stream_events(
        self, events: Iterator[object], turn_id: str
    ) -> Iterator[sidecar_pb2.RunTurnChunk]:
        index = 0
        for event in events:
            if isinstance(event, TextStreamEvent):
                for piece in chunk_text(event.text):
                    if CHUNK_DELAY_SEC > 0:
                        time.sleep(CHUNK_DELAY_SEC)
                    yield sidecar_pb2.RunTurnChunk(
                        **_chunk_kwargs(
                            TextStreamEvent(text=piece, sequence=event.sequence),
                            index=index,
                            turn_id=turn_id,
                        )
                    )
                    index += 1
                continue
            yield sidecar_pb2.RunTurnChunk(
                **_chunk_kwargs(event, index=index, turn_id=turn_id)
            )
            index += 1
        yield sidecar_pb2.RunTurnChunk(
            text="",
            index=index,
            done=True,
            turn_id=turn_id,
        )


def add_to_server(servicer: AgentServicer, server: grpc.Server) -> None:
    sidecar_pb2_grpc.add_SidecarServiceServicer_to_server(servicer, server)

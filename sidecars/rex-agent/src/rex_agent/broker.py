"""Broker client: daemon UDS gRPC (rex.v1)."""

from __future__ import annotations

import grpc

from rex_agent.config import daemon_socket

try:
    from rex.v1 import rex_pb2, rex_pb2_grpc
except ImportError as exc:  # pragma: no cover
    raise ImportError(
        "rex.v1 protobuf stubs not found. Run `rex proto install` and set "
        "PYTHONPATH to $(rex proto path)."
    ) from exc


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


def broker_inference(
    prompt: str,
    mode: str,
    model: str,
    turn_id: str | None = None,
) -> tuple[bool, str]:
    """
    Call BrokerInference on the daemon.

    Returns (ok, text_or_error_message).
    """
    socket = daemon_socket()
    channel = _daemon_channel(socket)
    try:
        stub = rex_pb2_grpc.RexServiceStub(channel)
        request = rex_pb2.BrokerInferenceRequest(
            prompt=prompt,
            mode=mode or "ask",
            model=model or "",
        )
        try:
            response = stub.BrokerInference(
                request,
                timeout=30.0,
                metadata=_metadata(turn_id),
            )
        except grpc.RpcError as err:
            return False, str(err)
        if response.ok:
            return True, response.text
        return False, response.error or "broker inference failed"
    finally:
        channel.close()

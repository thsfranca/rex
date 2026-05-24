"""Entrypoint for the rex-agent sidecar binary."""

from __future__ import annotations

import signal
import sys
from concurrent import futures
from pathlib import Path

import grpc

from rex_agent import __version__
from rex_agent.config import sidecar_socket
from rex_agent.server import AgentServicer, add_to_server


def remove_stale_socket(path: str) -> None:
    socket_path = Path(path)
    if socket_path.exists():
        socket_path.unlink()


def serve(socket_path: str) -> None:
    remove_stale_socket(socket_path)
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=4))
    add_to_server(AgentServicer(), server)
    server.add_insecure_port(f"unix://{socket_path}")
    server.start()
    sys.stderr.write(
        f"rex-agent event=listen socket={socket_path} version={__version__}\n"
    )
    sys.stderr.flush()

    stopped = False

    def _shutdown(_signum, _frame) -> None:
        nonlocal stopped
        if stopped:
            return
        stopped = True
        sys.stderr.write(
            f"rex-agent event=shutdown socket={socket_path} reason=signal\n"
        )
        sys.stderr.flush()
        server.stop(grace=1)

    signal.signal(signal.SIGINT, _shutdown)
    signal.signal(signal.SIGTERM, _shutdown)
    server.wait_for_termination()
    remove_stale_socket(socket_path)


def main() -> None:
    serve(sidecar_socket())


if __name__ == "__main__":
    main()

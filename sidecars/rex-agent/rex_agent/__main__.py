"""Entrypoint for rex-agent sidecar."""

from __future__ import annotations

from rex_agent.config import bootstrap_proto_path


def main() -> None:
    bootstrap_proto_path()
    from rex_agent.server import serve

    serve()


if __name__ == "__main__":
    main()

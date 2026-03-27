from __future__ import annotations

import argparse

import uvicorn

from app.config import ServerConfig, load_config


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="rex",
        description="OpenAI-compatible proxy that routes requests to the cheapest available model.",
    )
    parser.add_argument("--host", type=str, default=None)
    parser.add_argument("--port", type=int, default=None)
    parser.add_argument("--config", type=str, default="config.yaml")
    return parser.parse_args()


def main() -> None:
    args = _parse_args()

    config = load_config(args.config)
    server = config.server if config else ServerConfig()

    host = args.host or server.host
    port = args.port or server.port

    uvicorn.run("app.main:app", host=host, port=port)

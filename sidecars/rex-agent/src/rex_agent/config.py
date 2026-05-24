"""Resolve sidecar and daemon socket paths from environment and Rex config."""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any, Optional

DEFAULT_DAEMON_SOCKET = "/tmp/rex.sock"
DEFAULT_SIDECAR_SOCKET = "/tmp/rex-sidecar.sock"
REX_ROOT_ENV = "REX_ROOT"


def rex_root() -> Path:
    raw = os.environ.get(REX_ROOT_ENV, "").strip()
    if raw:
        return Path(raw).expanduser()
    return Path.home() / ".rex"


def _load_config_json() -> Optional[dict[str, Any]]:
    path = rex_root() / "config.json"
    if not path.is_file():
        return None
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None


def sidecar_socket() -> str:
    env = os.environ.get("REX_SIDECAR_SOCKET", "").strip()
    if env:
        return env
    cfg = _load_config_json()
    if cfg:
        active = (cfg.get("sidecars") or {}).get("active", "")
        for entry in (cfg.get("sidecars") or {}).get("list") or []:
            if entry.get("name") == active and entry.get("socket"):
                return str(entry["socket"])
        for entry in (cfg.get("sidecars") or {}).get("list") or []:
            if entry.get("socket"):
                return str(entry["socket"])
    return DEFAULT_SIDECAR_SOCKET


def daemon_socket() -> str:
    env = os.environ.get("REX_DAEMON_SOCKET", "").strip()
    if env:
        return env
    cfg = _load_config_json()
    if cfg:
        daemon = cfg.get("daemon") or {}
        if daemon.get("socket"):
            return str(daemon["socket"])
    return DEFAULT_DAEMON_SOCKET

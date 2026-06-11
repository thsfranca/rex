"""Resolve sidecar and daemon socket paths from environment and Rex config."""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any, Optional

DEFAULT_DAEMON_SOCKET = "/tmp/rex.sock"
DEFAULT_SIDECAR_SOCKET = "/tmp/rex-sidecar.sock"
DEFAULT_MAX_TOOL_STEPS = 12
DEFAULT_MAX_TOOL_RESULT_BYTES = 8192
DEFAULT_COMPACTION_SUFFIX_FRACTION = 0.25
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


def max_tool_steps() -> int:
    return max_tool_steps_for_mode("agent")


def max_tool_steps_for_mode(mode: str) -> int:
    normalized = (mode or "ask").strip().lower() or "ask"
    cfg = _load_config_json()
    if cfg:
        agent = cfg.get("agent") or {}
        if normalized == "ask":
            steps = agent.get("max_tool_steps_ask")
            if isinstance(steps, int) and steps > 0:
                return steps
        steps = agent.get("max_tool_steps")
        if isinstance(steps, int) and steps > 0:
            return steps
    if normalized == "ask":
        return 5
    return DEFAULT_MAX_TOOL_STEPS


def max_tool_result_bytes() -> int:
    cfg = _load_config_json()
    if cfg:
        broker = cfg.get("broker") or {}
        limit = broker.get("max_tool_result_bytes")
        if isinstance(limit, int) and limit > 0:
            return limit
    return DEFAULT_MAX_TOOL_RESULT_BYTES


def compaction_suffix_fraction() -> float:
    cfg = _load_config_json()
    if cfg:
        agent = cfg.get("agent") or {}
        fraction = agent.get("compaction_suffix_fraction")
        if isinstance(fraction, (int, float)) and 0 < float(fraction) < 1:
            return float(fraction)
    return DEFAULT_COMPACTION_SUFFIX_FRACTION


def read_pruning_enabled() -> bool:
    cfg = _load_config_json()
    if cfg:
        agent = cfg.get("agent") or {}
        flag = agent.get("read_pruning_enabled")
        if isinstance(flag, bool):
            return flag
    return False

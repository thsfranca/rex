"""Resolve sidecar and daemon socket paths from environment and Rex config."""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any, Optional

DEFAULT_DAEMON_SOCKET = "/tmp/rex.sock"
DEFAULT_SIDECAR_SOCKET = "/tmp/rex-sidecar.sock"
DEFAULT_MAX_TOOL_STEPS = 25
DEFAULT_MAX_TOOL_STEPS_ASK = 15
DEFAULT_MAX_TOOL_STEPS_PLAN = 25
DEFAULT_MAX_TOOLS_PER_STEP = 8
DEFAULT_MAX_TOOL_RESULT_BYTES = 8192
DEFAULT_COMPACTION_SUFFIX_FRACTION = 0.25
DEFAULT_COMPACTION_ENABLED = False
DEFAULT_DETERMINISTIC_INIT_ENABLED = True
DEFAULT_SOFT_CAP_ENABLED = True
DEFAULT_SOFT_CAP_FRACTION = 2 / 3
DEFAULT_SOFT_CAP_STEP_EXTENSION = 10
DEFAULT_BROKER_TIMEOUT_SEC = 120.0
REX_ROOT_ENV = "REX_ROOT"


def rex_root() -> Path:
    raw = os.environ.get(REX_ROOT_ENV, "").strip()
    if raw:
        return Path(raw).expanduser()
    return Path.home() / ".rex"


def _read_config_file(path: Path) -> Optional[dict[str, Any]]:
    if not path.is_file():
        return None
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    return data if isinstance(data, dict) else None


def _deep_merge_dict(base: dict[str, Any], overlay: dict[str, Any]) -> dict[str, Any]:
    merged = dict(base)
    for key, value in overlay.items():
        if (
            key in merged
            and isinstance(merged[key], dict)
            and isinstance(value, dict)
        ):
            merged[key] = _deep_merge_dict(merged[key], value)
        else:
            merged[key] = value
    return merged


def _project_config_path(workspace_root: str) -> Optional[Path]:
    trimmed = workspace_root.strip()
    if not trimmed:
        return None
    candidate = Path(trimmed).expanduser() / ".rex" / "config.json"
    return candidate if candidate.is_file() else None


def load_merged_config() -> dict[str, Any]:
    """Merge $REX_ROOT/config.json with project .rex/config.json (via workspace.root)."""
    effective: dict[str, Any] = {}
    global_cfg = _read_config_file(rex_root() / "config.json")
    if global_cfg:
        effective = _deep_merge_dict(effective, global_cfg)
    workspace = effective.get("workspace")
    workspace_root = ""
    if isinstance(workspace, dict):
        workspace_root = str(workspace.get("root") or "")
    project_path = _project_config_path(workspace_root)
    if project_path is not None:
        project_cfg = _read_config_file(project_path)
        if project_cfg:
            effective = _deep_merge_dict(effective, project_cfg)
    return effective


def _load_config_json() -> Optional[dict[str, Any]]:
    merged = load_merged_config()
    return merged or None


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
            return DEFAULT_MAX_TOOL_STEPS_ASK
        if normalized == "plan":
            steps = agent.get("max_tool_steps_plan")
            if isinstance(steps, int) and steps > 0:
                return steps
            return DEFAULT_MAX_TOOL_STEPS_PLAN
        steps = agent.get("max_tool_steps")
        if isinstance(steps, int) and steps > 0:
            return steps
    if normalized == "ask":
        return DEFAULT_MAX_TOOL_STEPS_ASK
    if normalized == "plan":
        return DEFAULT_MAX_TOOL_STEPS_PLAN
    return DEFAULT_MAX_TOOL_STEPS


def max_tools_per_step() -> int:
    cfg = _load_config_json()
    if cfg:
        agent = cfg.get("agent") or {}
        cap = agent.get("max_tools_per_step")
        if isinstance(cap, int) and cap > 0:
            return cap
    return DEFAULT_MAX_TOOLS_PER_STEP


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


def search_enabled() -> bool:
    cfg = _load_config_json()
    if cfg:
        search = cfg.get("search") or {}
        enabled = search.get("enabled")
        if isinstance(enabled, bool):
            return enabled
    return False


def deterministic_init_enabled() -> bool:
    cfg = _load_config_json()
    if cfg:
        agent = cfg.get("agent") or {}
        flag = agent.get("deterministic_init_enabled")
        if isinstance(flag, bool):
            return flag
    return DEFAULT_DETERMINISTIC_INIT_ENABLED


def compaction_enabled() -> bool:
    cfg = _load_config_json()
    if cfg:
        agent = cfg.get("agent") or {}
        flag = agent.get("compaction_enabled")
        if isinstance(flag, bool):
            return flag
    return DEFAULT_COMPACTION_ENABLED


def soft_cap_enabled() -> bool:
    cfg = _load_config_json()
    if cfg:
        agent = cfg.get("agent") or {}
        flag = agent.get("soft_cap_enabled")
        if isinstance(flag, bool):
            return flag
    return DEFAULT_SOFT_CAP_ENABLED


def soft_cap_fraction() -> float:
    cfg = _load_config_json()
    if cfg:
        agent = cfg.get("agent") or {}
        fraction = agent.get("soft_cap_fraction")
        if isinstance(fraction, (int, float)) and 0 < float(fraction) < 1:
            return float(fraction)
    return DEFAULT_SOFT_CAP_FRACTION


def soft_cap_step_extension() -> int:
    cfg = _load_config_json()
    if cfg:
        agent = cfg.get("agent") or {}
        bonus = agent.get("soft_cap_step_extension")
        if isinstance(bonus, int) and bonus > 0:
            return bonus
    return DEFAULT_SOFT_CAP_STEP_EXTENSION


def soft_cap_threshold(max_steps: int) -> int:
    return max(1, int(max_steps * soft_cap_fraction()))


def broker_timeout_secs() -> float:
    """Match sidecar broker gRPC budget to daemon HTTP inference timeout."""
    cfg = _load_config_json()
    if cfg:
        inference = cfg.get("inference") or {}
        openai_compat = inference.get("openai_compat") or {}
        timeout = openai_compat.get("timeout_secs")
        if isinstance(timeout, (int, float)) and float(timeout) > 0:
            return float(timeout)
        cursor_cli = inference.get("cursor_cli") or {}
        cursor_timeout = cursor_cli.get("timeout_secs")
        if isinstance(cursor_timeout, (int, float)) and float(cursor_timeout) > 0:
            return float(cursor_timeout)
    return DEFAULT_BROKER_TIMEOUT_SEC

"""Load merged Rex JSON config and bootstrap generated proto imports."""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path
from typing import Any

DEFAULT_REX_HOME = Path.home() / ".rex"
DEFAULT_GEN_ROOT = DEFAULT_REX_HOME / "proto" / "gen"


def rex_home() -> Path:
    raw = os.environ.get("REX_HOME", "").strip()
    return Path(raw).expanduser() if raw else DEFAULT_REX_HOME


def user_config_path() -> Path:
    override = os.environ.get("REX_CONFIG", "").strip()
    if override:
        return Path(override).expanduser()
    return rex_home() / "config.json"


def find_project_config() -> Path | None:
    cwd = Path.cwd()
    for parent in [cwd, *cwd.parents]:
        candidate = parent / ".rex" / "config.json"
        if candidate.is_file():
            return candidate
    return None


def load_merged_config() -> dict[str, Any]:
    config: dict[str, Any] = _default_config()
    user_path = user_config_path()
    if user_path.is_file():
        _deep_merge(config, _read_json(user_path))
    project = find_project_config()
    if project is not None:
        _deep_merge(config, _read_json(project))
    _apply_env_overrides(config)
    return config


def bootstrap_proto_path() -> None:
    cfg = load_merged_config()
    gen_root = Path(cfg["proto"]["gen_root"]).expanduser().resolve()
    python_path = gen_root / "python"
    if str(python_path) not in sys.path:
        sys.path.insert(0, str(python_path))


def daemon_socket(cfg: dict[str, Any] | None = None) -> str:
    data = cfg or load_merged_config()
    return str(data["daemon"]["socket"])


def sidecar_socket(cfg: dict[str, Any] | None = None) -> str:
    data = cfg or load_merged_config()
    active = data["sidecars"]["active"]
    for entry in data["sidecars"]["list"]:
        if entry["name"] == active:
            return str(entry["socket"])
    return "/tmp/rex-sidecar.sock"


def max_tool_steps(cfg: dict[str, Any] | None = None) -> int:
    data = cfg or load_merged_config()
    return int(data.get("agent", {}).get("max_tool_steps", 8))


def _default_config() -> dict[str, Any]:
    return {
        "version": 1,
        "daemon": {"socket": "/tmp/rex.sock"},
        "sidecars": {
            "active": "agent",
            "list": [{"name": "agent", "socket": "/tmp/rex-sidecar.sock"}],
        },
        "proto": {"gen_root": str(DEFAULT_GEN_ROOT)},
        "agent": {"max_tool_steps": 8},
    }


def _read_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _deep_merge(base: dict[str, Any], overlay: dict[str, Any]) -> None:
    for key, value in overlay.items():
        if isinstance(value, dict) and isinstance(base.get(key), dict):
            _deep_merge(base[key], value)
        else:
            base[key] = value


def _apply_env_overrides(config: dict[str, Any]) -> None:
    if v := os.environ.get("REX_DAEMON_SOCKET", "").strip():
        config.setdefault("daemon", {})["socket"] = v
    if v := os.environ.get("REX_SIDECAR_SOCKET", "").strip():
        active = config.get("sidecars", {}).get("active", "agent")
        for entry in config.get("sidecars", {}).get("list", []):
            if entry.get("name") == active:
                entry["socket"] = v
    if v := os.environ.get("REX_AGENT_MAX_TOOL_STEPS", "").strip():
        config.setdefault("agent", {})["max_tool_steps"] = int(v)

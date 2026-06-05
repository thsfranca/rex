import json
from pathlib import Path

import pytest

from rex_agent import config


def test_sidecar_socket_env_overrides_config(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    root = tmp_path / "rex"
    root.mkdir()
    (root / "config.json").write_text(
        json.dumps(
            {
                "sidecars": {
                    "active": "agent",
                    "list": [{"name": "agent", "socket": "/from/config.sock"}],
                }
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setenv(config.REX_ROOT_ENV, str(root))
    monkeypatch.setenv("REX_SIDECAR_SOCKET", "/from/env.sock")
    assert config.sidecar_socket() == "/from/env.sock"


def test_daemon_socket_from_config(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    root = tmp_path / "rex"
    root.mkdir()
    (root / "config.json").write_text(
        json.dumps({"daemon": {"socket": "/daemon/config.sock"}}),
        encoding="utf-8",
    )
    monkeypatch.setenv(config.REX_ROOT_ENV, str(root))
    monkeypatch.delenv("REX_DAEMON_SOCKET", raising=False)
    assert config.daemon_socket() == "/daemon/config.sock"


def test_defaults_without_config(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv(config.REX_ROOT_ENV, "/nonexistent-rex-root-for-test")
    monkeypatch.delenv("REX_SIDECAR_SOCKET", raising=False)
    monkeypatch.delenv("REX_DAEMON_SOCKET", raising=False)
    assert config.sidecar_socket() == config.DEFAULT_SIDECAR_SOCKET
    assert config.daemon_socket() == config.DEFAULT_DAEMON_SOCKET


def test_agent_limits_from_config(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    root = tmp_path / "rex"
    root.mkdir()
    (root / "config.json").write_text(
        json.dumps(
            {
                "agent": {"max_tool_steps": 3},
                "broker": {"max_tool_result_bytes": 1024},
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setenv(config.REX_ROOT_ENV, str(root))
    assert config.max_tool_steps() == 3
    assert config.max_tool_result_bytes() == 1024


def test_agent_limits_defaults(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv(config.REX_ROOT_ENV, "/nonexistent-rex-root-for-test")
    assert config.max_tool_steps() == config.DEFAULT_MAX_TOOL_STEPS
    assert config.max_tool_result_bytes() == config.DEFAULT_MAX_TOOL_RESULT_BYTES


def test_read_pruning_enabled_from_config(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    root = tmp_path / "rex"
    root.mkdir()
    (root / "config.json").write_text(
        json.dumps({"agent": {"read_pruning_enabled": True}}),
        encoding="utf-8",
    )
    monkeypatch.setenv(config.REX_ROOT_ENV, str(root))
    assert config.read_pruning_enabled() is True


def test_read_pruning_disabled_by_default(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv(config.REX_ROOT_ENV, "/nonexistent-rex-root-for-test")
    assert config.read_pruning_enabled() is False

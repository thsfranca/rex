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


def test_broker_timeout_reads_openai_compat_timeout(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    root = tmp_path / "rex"
    root.mkdir()
    (root / "config.json").write_text(
        json.dumps({"inference": {"openai_compat": {"timeout_secs": 90}}}),
        encoding="utf-8",
    )
    monkeypatch.setenv(config.REX_ROOT_ENV, str(root))
    assert config.broker_timeout_secs() == 90.0


def test_broker_timeout_default_when_unconfigured(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    root = tmp_path / "rex"
    root.mkdir()
    (root / "config.json").write_text("{}", encoding="utf-8")
    monkeypatch.setenv(config.REX_ROOT_ENV, str(root))
    assert config.broker_timeout_secs() == config.DEFAULT_BROKER_TIMEOUT_SEC


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
    assert config.max_tool_steps() == 0
    assert config.max_tool_result_bytes() == 1024


def test_ask_tool_steps_deprecated_r069(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    root = tmp_path / "rex"
    root.mkdir()
    (root / "config.json").write_text(
        json.dumps({"agent": {"max_tool_steps": 12}}),
        encoding="utf-8",
    )
    monkeypatch.setenv(config.REX_ROOT_ENV, str(root))
    assert config.max_tool_steps_for_mode("ask") == 0
    assert config.max_tool_steps_for_mode("agent") == 0


def test_ask_tool_steps_config_ignored_r069(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    root = tmp_path / "rex"
    root.mkdir()
    (root / "config.json").write_text(
        json.dumps({"agent": {"max_tool_steps": 12, "max_tool_steps_ask": 2}}),
        encoding="utf-8",
    )
    monkeypatch.setenv(config.REX_ROOT_ENV, str(root))
    assert config.max_tool_steps_for_mode("ask") == 0
    assert config.max_tool_steps_for_mode("agent") == 0


def test_plan_tool_steps_deprecated_r069(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    root = tmp_path / "rex"
    root.mkdir()
    (root / "config.json").write_text(
        json.dumps({"agent": {"max_tool_steps": 12}}),
        encoding="utf-8",
    )
    monkeypatch.setenv(config.REX_ROOT_ENV, str(root))
    assert config.max_tool_steps_for_mode("plan") == 0
    assert config.max_tool_steps_for_mode("agent") == 0


def test_plan_tool_steps_config_ignored_r069(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    root = tmp_path / "rex"
    root.mkdir()
    (root / "config.json").write_text(
        json.dumps({"agent": {"max_tool_steps_plan": 15}}),
        encoding="utf-8",
    )
    monkeypatch.setenv(config.REX_ROOT_ENV, str(root))
    assert config.max_tool_steps_for_mode("plan") == 0


def test_agent_limits_defaults(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv(config.REX_ROOT_ENV, "/nonexistent-rex-root-for-test")
    assert config.max_tool_steps() == 0
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


def test_max_tools_per_step_from_config(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    root = tmp_path / "rex"
    root.mkdir()
    (root / "config.json").write_text(
        json.dumps({"agent": {"max_tools_per_step": 3}}),
        encoding="utf-8",
    )
    monkeypatch.setenv(config.REX_ROOT_ENV, str(root))
    assert config.max_tools_per_step() == 3


def test_max_tools_per_step_default(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv(config.REX_ROOT_ENV, "/nonexistent-rex-root-for-test")
    assert config.max_tools_per_step() == config.DEFAULT_MAX_TOOLS_PER_STEP


def test_search_enabled_from_merged_config(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    root = tmp_path / "rex"
    root.mkdir()
    project = tmp_path / "project"
    project.mkdir()
    (project / ".rex").mkdir()
    (root / "config.json").write_text(
        json.dumps({"workspace": {"root": str(project)}}),
        encoding="utf-8",
    )
    (project / ".rex" / "config.json").write_text(
        json.dumps({"search": {"enabled": True, "provider": "mock"}}),
        encoding="utf-8",
    )
    monkeypatch.setenv(config.REX_ROOT_ENV, str(root))
    assert config.search_enabled() is True


def test_search_disabled_by_default(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv(config.REX_ROOT_ENV, "/nonexistent-rex-root-for-test")
    assert config.search_enabled() is False


def test_ask_step_cap_removed_r069(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv(config.REX_ROOT_ENV, "/nonexistent-rex-root-for-test")
    assert config.max_tool_steps_for_mode("ask") == 0


def test_deterministic_init_enabled_default(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv(config.REX_ROOT_ENV, "/nonexistent-rex-root-for-test")
    assert config.deterministic_init_enabled() is True


def test_deterministic_init_disabled_from_config(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    root = tmp_path / "rex"
    root.mkdir()
    (root / "config.json").write_text(
        json.dumps({"agent": {"deterministic_init_enabled": False}}),
        encoding="utf-8",
    )
    monkeypatch.setenv(config.REX_ROOT_ENV, str(root))
    assert config.deterministic_init_enabled() is False


def test_compaction_disabled_by_default(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv(config.REX_ROOT_ENV, "/nonexistent-rex-root-for-test")
    assert config.compaction_enabled() is False


def test_compaction_enabled_from_config(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    root = tmp_path / "rex"
    root.mkdir()
    (root / "config.json").write_text(
        json.dumps({"agent": {"compaction_enabled": True}}),
        encoding="utf-8",
    )
    monkeypatch.setenv(config.REX_ROOT_ENV, str(root))
    assert config.compaction_enabled() is True


def test_soft_cap_enabled_by_default(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv(config.REX_ROOT_ENV, "/nonexistent-rex-root-for-test")
    assert config.soft_cap_enabled() is True


def test_soft_cap_fraction_default(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv(config.REX_ROOT_ENV, "/nonexistent-rex-root-for-test")
    assert config.soft_cap_fraction() == pytest.approx(config.DEFAULT_SOFT_CAP_FRACTION)

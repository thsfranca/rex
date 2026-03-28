from __future__ import annotations

import argparse
import os
import signal
import sys
from unittest.mock import MagicMock, patch

import pytest

from app.cli import (
    _client_base_url,
    _client_connect_host,
    _is_process_running,
    _read_pid,
    _wait_for_ready,
    _write_pid,
    cmd_reset,
    cmd_start,
    cmd_stop,
)


class TestHelpers:
    def test_client_connect_host(self):
        assert _client_connect_host("0.0.0.0") == "127.0.0.1"
        assert _client_connect_host("localhost") == "127.0.0.1"
        assert _client_connect_host("127.0.0.1") == "127.0.0.1"
        assert _client_connect_host("192.168.1.1") == "192.168.1.1"

    def test_client_base_url(self):
        assert _client_base_url(8000, "127.0.0.1") == "http://127.0.0.1:8000"
        assert _client_base_url(9000, "127.0.0.1") == "http://127.0.0.1:9000"
        assert _client_base_url(8000, "0.0.0.0") == "http://127.0.0.1:8000"
        assert _client_base_url(8000, "localhost") == "http://127.0.0.1:8000"

    @patch("app.cli.httpx.get")
    def test_wait_for_ready_calls_health(self, mock_get):
        mock_get.return_value = MagicMock(status_code=200)
        assert _wait_for_ready(8000, "127.0.0.1") is True
        mock_get.assert_called_once()
        assert mock_get.call_args[0][0] == "http://127.0.0.1:8000/health"

    def test_is_process_running_current_process(self):
        assert _is_process_running(os.getpid()) is True

    def test_is_process_running_nonexistent(self):
        assert _is_process_running(99999999) is False

    def test_write_and_read_pid(self, tmp_path):
        pid_file = tmp_path / "rex.pid"
        with patch("app.cli.PID_FILE", pid_file):
            _write_pid(os.getpid())
            assert pid_file.exists()
            assert _read_pid() == os.getpid()

    def test_read_pid_no_file(self, tmp_path):
        pid_file = tmp_path / "rex.pid"
        with patch("app.cli.PID_FILE", pid_file):
            assert _read_pid() is None

    def test_read_pid_stale_process(self, tmp_path):
        pid_file = tmp_path / "rex.pid"
        pid_file.write_text("99999999")
        with patch("app.cli.PID_FILE", pid_file):
            assert _read_pid() is None
            assert not pid_file.exists()

    def test_read_pid_invalid_content(self, tmp_path):
        pid_file = tmp_path / "rex.pid"
        pid_file.write_text("not-a-number")
        with patch("app.cli.PID_FILE", pid_file):
            assert _read_pid() is None
            assert not pid_file.exists()


class TestCmdStart:
    @patch("app.cli._wait_for_ready", return_value=True)
    @patch("app.cli.subprocess.Popen")
    @patch("app.cli._read_pid", return_value=None)
    def test_starts_background_process(self, mock_read, mock_popen, mock_wait, tmp_path, capsys):
        pid_file = tmp_path / "rex.pid"
        mock_process = MagicMock()
        mock_process.pid = 12345
        mock_popen.return_value = mock_process

        with patch("app.cli.PID_FILE", pid_file):
            args = argparse.Namespace(host="0.0.0.0", port=8000)
            cmd_start(args)

        mock_popen.assert_called_once()
        popen_cmd = mock_popen.call_args[0][0]
        assert popen_cmd[0] == sys.executable
        assert popen_cmd[1:4] == ["-m", "uvicorn", "app.main:app"]
        assert "--host" in popen_cmd
        assert "--port" in popen_cmd
        assert "0.0.0.0" in popen_cmd
        assert "8000" in popen_cmd
        assert "--ssl-certfile" not in popen_cmd
        assert "--ssl-keyfile" not in popen_cmd
        assert pid_file.read_text() == "12345"
        captured = capsys.readouterr()
        assert "12345" in captured.out
        assert "8000" in captured.out

    @patch("app.cli._read_pid", return_value=42)
    def test_exits_if_already_running(self, mock_read):
        args = argparse.Namespace(host="0.0.0.0", port=8000)
        with pytest.raises(SystemExit, match="1"):
            cmd_start(args)

    @patch("app.cli._wait_for_ready", return_value=False)
    @patch("app.cli.subprocess.Popen")
    @patch("app.cli._read_pid", return_value=None)
    def test_exits_if_health_check_fails(self, mock_read, mock_popen, mock_wait, tmp_path):
        pid_file = tmp_path / "rex.pid"
        mock_process = MagicMock()
        mock_process.pid = 12345
        mock_popen.return_value = mock_process

        with patch("app.cli.PID_FILE", pid_file):
            args = argparse.Namespace(host="0.0.0.0", port=8000)
            with pytest.raises(SystemExit, match="1"):
                cmd_start(args)


class TestCmdStop:
    @patch("app.cli.os.kill")
    @patch("app.cli._is_process_running", return_value=False)
    @patch("app.cli._read_pid", return_value=12345)
    def test_stops_running_process(self, mock_read, mock_running, mock_kill, tmp_path, capsys):
        pid_file = tmp_path / "rex.pid"
        pid_file.write_text("12345")
        with patch("app.cli.PID_FILE", pid_file):
            cmd_stop(argparse.Namespace())

        mock_kill.assert_called_once_with(12345, signal.SIGTERM)
        captured = capsys.readouterr()
        assert "stopped" in captured.out

    @patch("app.cli._read_pid", return_value=None)
    def test_exits_if_not_running(self, mock_read):
        with pytest.raises(SystemExit, match="1"):
            cmd_stop(argparse.Namespace())


class TestCmdReset:
    @patch("app.cli.httpx.post")
    @patch("app.cli._read_pid", return_value=12345)
    def test_resets_with_yes_flag(self, mock_read, mock_post, capsys):
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_post.return_value = mock_response

        args = argparse.Namespace(yes=True, host="127.0.0.1", port=8000)
        cmd_reset(args)

        mock_post.assert_called_once_with("http://127.0.0.1:8000/v1/reset", timeout=10.0)
        captured = capsys.readouterr()
        assert "cleared" in captured.out

    @patch("app.cli.httpx.post")
    @patch("builtins.input", return_value="y")
    @patch("app.cli._read_pid", return_value=12345)
    def test_resets_with_confirmation(self, mock_read, mock_input, mock_post, capsys):
        mock_response = MagicMock()
        mock_response.status_code = 200
        mock_post.return_value = mock_response

        args = argparse.Namespace(yes=False, host="127.0.0.1", port=8000)
        cmd_reset(args)

        mock_input.assert_called_once()
        mock_post.assert_called_once()

    @patch("builtins.input", return_value="n")
    @patch("app.cli._read_pid", return_value=12345)
    def test_aborts_when_denied(self, mock_read, mock_input, capsys):
        args = argparse.Namespace(yes=False, host="127.0.0.1", port=8000)
        with pytest.raises(SystemExit, match="0"):
            cmd_reset(args)

    @patch("app.cli._read_pid", return_value=None)
    def test_exits_if_not_running(self, mock_read):
        args = argparse.Namespace(yes=True, host="127.0.0.1", port=8000)
        with pytest.raises(SystemExit, match="1"):
            cmd_reset(args)

    @patch("app.cli.httpx.post")
    @patch("app.cli._read_pid", return_value=12345)
    def test_exits_on_failed_reset(self, mock_read, mock_post):
        mock_response = MagicMock()
        mock_response.status_code = 500
        mock_response.text = "Internal error"
        mock_post.return_value = mock_response

        args = argparse.Namespace(yes=True, host="127.0.0.1", port=8000)
        with pytest.raises(SystemExit, match="1"):
            cmd_reset(args)

    @patch("app.cli.httpx.post", side_effect=__import__("httpx").ConnectError("refused"))
    @patch("app.cli._read_pid", return_value=12345)
    def test_exits_on_connection_error(self, mock_read, mock_post):
        args = argparse.Namespace(yes=True, host="127.0.0.1", port=8000)
        with pytest.raises(SystemExit, match="1"):
            cmd_reset(args)

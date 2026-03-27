from __future__ import annotations

import argparse
import os
import signal
import subprocess
import sys
import time
from pathlib import Path

import httpx

PID_FILE = Path("~/.rex/rex.pid").expanduser()
DEFAULT_HOST = "0.0.0.0"
DEFAULT_PORT = 8000


def _get_base_url(port: int) -> str:
    return f"http://localhost:{port}"


def _is_process_running(pid: int) -> bool:
    try:
        os.kill(pid, 0)
        return True
    except OSError:
        return False


def _read_pid() -> int | None:
    if not PID_FILE.exists():
        return None
    try:
        pid = int(PID_FILE.read_text().strip())
        if _is_process_running(pid):
            return pid
        PID_FILE.unlink(missing_ok=True)
        return None
    except (ValueError, OSError):
        PID_FILE.unlink(missing_ok=True)
        return None


def _write_pid(pid: int) -> None:
    PID_FILE.parent.mkdir(parents=True, exist_ok=True)
    PID_FILE.write_text(str(pid))


def _wait_for_ready(port: int, timeout: float = 10.0) -> bool:
    deadline = time.monotonic() + timeout
    url = f"{_get_base_url(port)}/health"
    while time.monotonic() < deadline:
        try:
            resp = httpx.get(url, timeout=2.0)
            if resp.status_code == 200:
                return True
        except httpx.ConnectError:
            pass
        time.sleep(0.3)
    return False


def cmd_start(args: argparse.Namespace) -> None:
    port = args.port
    host = args.host

    existing_pid = _read_pid()
    if existing_pid is not None:
        print(f"Rex is already running (pid {existing_pid})")
        sys.exit(1)

    process = subprocess.Popen(
        [
            sys.executable,
            "-m",
            "uvicorn",
            "app.main:app",
            "--host",
            host,
            "--port",
            str(port),
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )

    _write_pid(process.pid)

    if _wait_for_ready(port):
        print(f"Rex started (pid {process.pid}) — listening on http://{host}:{port}/v1")
    else:
        print(f"Rex process started (pid {process.pid}) but health check timed out")
        sys.exit(1)


def cmd_stop(args: argparse.Namespace) -> None:
    pid = _read_pid()
    if pid is None:
        print("Rex is not running")
        sys.exit(1)

    os.kill(pid, signal.SIGTERM)

    deadline = time.monotonic() + 5.0
    while time.monotonic() < deadline:
        if not _is_process_running(pid):
            break
        time.sleep(0.2)

    PID_FILE.unlink(missing_ok=True)
    print(f"Rex stopped (pid {pid})")


def cmd_reset(args: argparse.Namespace) -> None:
    pid = _read_pid()
    if pid is None:
        print("Rex is not running — start it first with 'rex start'")
        sys.exit(1)

    if not args.yes:
        confirm = input("This will clear all learning data. Continue? [y/N] ")
        if confirm.lower() not in ("y", "yes"):
            print("Aborted")
            sys.exit(0)

    port = args.port
    url = f"{_get_base_url(port)}/v1/reset"
    try:
        resp = httpx.post(url, timeout=10.0)
        if resp.status_code == 200:
            print("All learning data cleared")
        else:
            print(f"Reset failed: {resp.text}")
            sys.exit(1)
    except httpx.ConnectError:
        print(f"Could not connect to Rex at localhost:{port}")
        sys.exit(1)


def main() -> None:
    parser = argparse.ArgumentParser(prog="rex", description="Rex — intelligent model router")
    subparsers = parser.add_subparsers(dest="command")

    start_parser = subparsers.add_parser("start", help="Start Rex as a background process")
    start_parser.add_argument("--host", default=DEFAULT_HOST, help="Host to bind to")
    start_parser.add_argument("--port", type=int, default=DEFAULT_PORT, help="Port to listen on")

    subparsers.add_parser("stop", help="Stop a running Rex instance")

    reset_parser = subparsers.add_parser("reset", help="Clear all learning data")
    reset_parser.add_argument("--yes", "-y", action="store_true", help="Skip confirmation prompt")
    reset_parser.add_argument(
        "--port", type=int, default=DEFAULT_PORT, help="Port Rex is running on"
    )

    args = parser.parse_args()

    if args.command is None:
        cmd_start(argparse.Namespace(host=DEFAULT_HOST, port=DEFAULT_PORT))
    elif args.command == "start":
        cmd_start(args)
    elif args.command == "stop":
        cmd_stop(args)
    elif args.command == "reset":
        cmd_reset(args)


if __name__ == "__main__":
    main()

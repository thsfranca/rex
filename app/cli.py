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


def _client_connect_host(bind_host: str) -> str:
    normalized = bind_host.strip().lower()
    if normalized in ("0.0.0.0", "localhost"):
        return "127.0.0.1"
    return bind_host


def _client_base_url(port: int, bind_host: str, *, use_tls: bool = False) -> str:
    scheme = "https" if use_tls else "http"
    host = _client_connect_host(bind_host)
    return f"{scheme}://{host}:{port}"


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


def _loopback_tls_health_verify(bind_host: str, use_tls: bool) -> bool:
    if not use_tls:
        return True
    return _client_connect_host(bind_host) != "127.0.0.1"


def _wait_for_ready(
    port: int,
    bind_host: str,
    *,
    use_tls: bool = False,
    timeout: float = 15.0,
    process: subprocess.Popen | None = None,
) -> bool:
    deadline = time.monotonic() + timeout
    url = f"{_client_base_url(port, bind_host, use_tls=use_tls)}/health"
    per_try = 5.0 if use_tls else 2.0
    verify = _loopback_tls_health_verify(bind_host, use_tls)
    while time.monotonic() < deadline:
        if process is not None and process.poll() is not None:
            return False
        try:
            resp = httpx.get(url, timeout=per_try, verify=verify)
            if resp.status_code == 200:
                return True
        except httpx.TransportError:
            pass
        time.sleep(0.3)
    return False


def cmd_start(args: argparse.Namespace) -> None:
    port = args.port
    host = args.host
    certfile = getattr(args, "certfile", None)
    keyfile = getattr(args, "keyfile", None)

    if (certfile is None) ^ (keyfile is None):
        print("error: --certfile and --keyfile must be given together for HTTPS (HTTP/2 via ALPN)")
        sys.exit(1)

    use_tls = certfile is not None and keyfile is not None

    existing_pid = _read_pid()
    if existing_pid is not None:
        print(f"Rex is already running (pid {existing_pid})")
        sys.exit(1)

    cmd = [
        sys.executable,
        "-m",
        "hypercorn",
        "app.main:app",
        "--bind",
        f"{host}:{port}",
    ]
    if use_tls:
        cmd.extend(["--certfile", certfile, "--keyfile", keyfile])

    process = subprocess.Popen(
        cmd,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )

    _write_pid(process.pid)

    if _wait_for_ready(port, host, use_tls=use_tls, process=process):
        scheme = "https" if use_tls else "http"
        print(f"Rex started (pid {process.pid}) — listening on {scheme}://{host}:{port}/v1")
    else:
        code = process.poll()
        if code is not None:
            PID_FILE.unlink(missing_ok=True)
            print(
                f"Hypercorn exited during startup (exit {code}). "
                f"Often port {port} is in use. Try: make stop, then lsof -i :{port}"
            )
        else:
            print(
                f"Rex process started (pid {process.pid}) but health check timed out "
                f"(HTTPS on a port that already serves plain HTTP can cause this)."
            )
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
    use_tls = getattr(args, "tls", False)
    reset_host = getattr(args, "host", "127.0.0.1")
    url = f"{_client_base_url(port, reset_host, use_tls=use_tls)}/v1/reset"
    try:
        resp = httpx.post(url, timeout=10.0)
        if resp.status_code == 200:
            print("All learning data cleared")
        else:
            print(f"Reset failed: {resp.text}")
            sys.exit(1)
    except httpx.ConnectError:
        print(f"Could not connect to Rex at {_client_connect_host(reset_host)}:{port}")
        sys.exit(1)


def main() -> None:
    parser = argparse.ArgumentParser(prog="rex", description="Rex — intelligent model router")
    subparsers = parser.add_subparsers(dest="command")

    start_parser = subparsers.add_parser("start", help="Start Rex as a background process")
    start_parser.add_argument("--host", default=DEFAULT_HOST, help="Host to bind to")
    start_parser.add_argument("--port", type=int, default=DEFAULT_PORT, help="Port to listen on")
    start_parser.add_argument(
        "--certfile",
        default=None,
        metavar="PATH",
        help="TLS certificate (requires --keyfile); enables HTTP/2 via ALPN",
    )
    start_parser.add_argument(
        "--keyfile",
        default=None,
        metavar="PATH",
        help="TLS private key (requires --certfile)",
    )

    subparsers.add_parser("stop", help="Stop a running Rex instance")

    reset_parser = subparsers.add_parser("reset", help="Clear all learning data")
    reset_parser.add_argument("--yes", "-y", action="store_true", help="Skip confirmation prompt")
    reset_parser.add_argument(
        "--host",
        default="127.0.0.1",
        help="Host Rex is bound to (for the reset request URL)",
    )
    reset_parser.add_argument(
        "--port", type=int, default=DEFAULT_PORT, help="Port Rex is running on"
    )
    reset_parser.add_argument(
        "--tls",
        action="store_true",
        help="Use https:// when Rex was started with --certfile and --keyfile",
    )

    args = parser.parse_args()

    if args.command is None:
        cmd_start(
            argparse.Namespace(
                host=DEFAULT_HOST,
                port=DEFAULT_PORT,
                certfile=None,
                keyfile=None,
            )
        )
    elif args.command == "start":
        cmd_start(args)
    elif args.command == "stop":
        cmd_stop(args)
    elif args.command == "reset":
        cmd_reset(args)


if __name__ == "__main__":
    main()

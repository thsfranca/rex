# rex-agent

Python sidecar implementing `rex.sidecar.v1` for the REX product agent program.

**R018:** LangGraph ReAct loops (`ask` / `plan` / `agent`) with broker-only `BrokerInference` and `Broker*` tools. **R017** gRPC server and incremental `RunTurn` streaming. **R044:** live event flush during graph execution via stream sink ([ADR 0030](../../docs/architecture/decisions/0030-incremental-run-turn-streaming.md)); ask-mode `web.search` when `search.enabled` ([ADR 0031](../../docs/architecture/decisions/0031-ask-mode-research-broker.md)).

## Prerequisites

- Python **3.10+** (macOS CLT default 3.9 is **not** supported)
- `rex` CLI on `PATH` (from this monorepo)
- `grpcio-tools` for `rex proto install` (see [docs/DEPENDENCIES.md](../../docs/DEPENDENCIES.md))

## Operator install (recommended)

From the Rex repository root (also run automatically by `./scripts/install-cli.sh`):

```bash
rex config init
./scripts/install-agent-sidecar.sh
rex sidecar doctor
```

This creates:

- **`$REX_ROOT/venv`** — editable `rex-agent` install (default `~/.rex/venv`)
- **`~/.cargo/bin/rex-agent`** — wrapper that sets `PYTHONPATH=$REX_ROOT/proto/gen` and runs the venv interpreter

Check dependencies first: `./scripts/install-preflight.sh`

## Maintainer install (manual venv)

When developing the sidecar package itself:

```bash
rex proto install
python3.12 -m venv .venv # or any Python >= 3.10
source .venv/bin/activate
pip install -e sidecars/rex-agent
export PYTHONPATH="$(rex proto path):${PYTHONPATH}"
```

Or use the dev launcher (no pip install): `sidecars/rex-agent/rex-agent` sets `PYTHONPATH` to proto gen + `src/`.

## Run

Daemon must be up for `RunTurn` to succeed:

```bash
rex-agent
```

Or let the desktop auto-start the daemon with the sidecar — set `sidecars.active` to an entry whose `binary` is `rex-agent` (see [DESIGN.md](DESIGN.md)).

## Configuration

| Variable | Purpose |
|----------|---------|
| `REX_SIDECAR_SOCKET` | Sidecar listen UDS (default `/tmp/rex-sidecar.sock` or config) |
| `REX_DAEMON_SOCKET` | Daemon broker UDS (default `/tmp/rex.sock` or config) |
| `REX_ROOT` | Config and proto gen root (default `~/.rex`) |

Supervisor injects `REX_ROOT`, `REX_SIDECAR_SOCKET`, `REX_DAEMON_SOCKET`, and `PYTHONPATH` (proto gen) when spawning the sidecar.

## Related

- [DESIGN.md](DESIGN.md) — capability contract
- [docs/AGENT_DELIVERY_ROADMAP.md](../../docs/AGENT_DELIVERY_ROADMAP.md) — R017–R019 **Done**
- [docs/OPERATOR_UX.md](../../docs/OPERATOR_UX.md) — live-model operator checklist

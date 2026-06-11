# rex-agent

Python sidecar implementing `rex.sidecar.v1` for the REX product agent program.

**R018:** LangGraph ReAct loops (`ask` / `plan` / `agent`) with broker-only `BrokerInference` and `Broker*` tools. **R017** gRPC server and incremental `RunTurn` streaming. **R044:** live event flush during graph execution via stream sink ([ADR 0028](../../docs/architecture/decisions/0028-incremental-run-turn-streaming.md)); ask-mode `web.search` when `search.enabled` ([ADR 0029](../../docs/architecture/decisions/0029-ask-mode-research-broker.md)).

## Prerequisites

- Python 3.10+
- `rex` CLI on `PATH` (from this monorepo)
- `grpcio-tools` for `rex proto install` (see [docs/DEPENDENCIES.md](../../docs/DEPENDENCIES.md))

## Bootstrap

From the Rex repository root:

```bash
rex config init
rex proto install
pip install -e sidecars/rex-agent
export PYTHONPATH="$(rex proto path):${PYTHONPATH}"
```

Run manually (daemon must be up for `RunTurn` to succeed):

```bash
rex-agent
```

Or let `rex daemon` supervise the sidecar — set `sidecars.active` to an entry whose `binary` is `rex-agent` (see [DESIGN.md](DESIGN.md)).

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
- [docs/EXTENSION_LOCAL_E2E.md](../../docs/EXTENSION_LOCAL_E2E.md) §8 — live-model operator checklist

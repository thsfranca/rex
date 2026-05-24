# rex-agent

Python sidecar for REX. Implements `rex.sidecar.v1` with a LangGraph ReAct loop and broker-only LLM/tools.

## Run locally

```bash
rex config init && rex proto install
cd sidecars/rex-agent
python3 -m venv .venv && source .venv/bin/activate
pip install -e .
rex daemon   # separate terminal
python -m rex_agent
```

Configuration is read from `$REX_HOME/config.json` (default `~/.rex/config.json`).

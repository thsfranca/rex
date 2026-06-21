# oMLX operator layout

Managed oMLX settings live in **`$REX_ROOT/config.json`** under `inference.omlx.*` — not environment variables.

- `inference.omlx.port` — loopback listen port (default **8000**)
- `inference.omlx.model_dir` — MLX-format model weights directory (optional; passed as `omlx serve --model-dir`)
- `inference.omlx.model` — default model id when `openai_compat.model` is unset

Run `rex omlx init` to materialize this directory and seed `config.snippet.json`; merge the snippet into your Rex config. Run `rex omlx doctor` for preflight checks.

Hub: [OMLX_INFERENCE.md](../../docs/OMLX_INFERENCE.md)

# oMLX operator layout

Managed oMLX uses `$REX_ROOT/omlx/` for optional environment overrides (`.env`).

- Point `OMLX_MODEL_DIR` at MLX-format model weights (subdirectories per model).
- Default listen port is **8000** (`OMLX_PORT`).
- Hub: [OMLX_INFERENCE.md](../../docs/OMLX_INFERENCE.md)

Run `rex omlx init` to materialize this directory; `rex omlx doctor` for preflight checks.

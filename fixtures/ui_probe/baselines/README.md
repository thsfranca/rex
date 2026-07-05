# UI probe baselines

Optional PNG baselines for `ui_diff_baseline` during local desktop harness runs.

## Refresh

1. `./scripts/setup_ui_probe_env.sh`
2. Open desktop harness session and navigate to the target scenario (for example `streaming`).
3. Capture via MCP `ui_diff_baseline` with `name` matching the scenario slug.

Baselines in this directory are **committed** when a scenario needs visual regression lock-in. The default harness CI path uses structural canvas asserts (`data-renderer`, motion tier, pixel drift) so CI does not require PNGs here.

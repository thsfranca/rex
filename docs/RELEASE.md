# Release automation

REX uses **two independent release planes**: the Rust workspace (`v*` tags) and the VS Code extension (`rex-vscode-v*` tags). Both use [Conventional Commits](../CONTRIBUTING.md) and open a **Release PR** on `main` before tagging.

## Core (Rust workspace)

| Item | Detail |
|------|--------|
| Version | `[workspace.package].version` in root [`Cargo.toml`](../Cargo.toml) |
| Changelog | [`CHANGELOG.md`](../CHANGELOG.md) |
| Release PR bot | [release-plz](https://release-plz.dev/) — workflow [`.github/workflows/release-plz.yml`](../.github/workflows/release-plz.yml), config [`release-plz.toml`](../release-plz.toml) |
| Tag | `vX.Y.Z` (for example `v0.1.1`) |
| Binaries | [cargo-dist](https://axodotdev.github.io/cargo-dist/) — workflow [`.github/workflows/release.yml`](../.github/workflows/release.yml) |
| Artifacts | `rex-cli`, `rex-daemon`, `rex-sidecar-stub` per target triple |

### Maintainer flow

1. Merge feature PRs to `main` with conventional commit messages (or squash titles that match [CONTRIBUTING.md](../CONTRIBUTING.md)).
2. Wait for **Release-plz** to open or update a Release PR (`release-core` label). Review version bump and [`CHANGELOG.md`](../CHANGELOG.md).
3. **Before `1.0.0`:** confirm every Must **RC-*** in [V1_0.md](V1_0.md) is **Met** — do not merge a Release PR that bumps to `1.0.0` until the gate is satisfied.
4. Merge the Release PR. release-plz creates the `v*` tag and a GitHub Release (notes only).
5. **Core Release** workflow runs on the tag: runs [`scripts/ci/run_rust_verify.sh`](../scripts/ci/run_rust_verify.sh), builds archives with cargo-dist, uploads assets to the same GitHub Release.

### Local binary dry-run

```bash
cargo install cargo-dist --locked   # once
./scripts/release/build_core_artifacts.sh v0.1.0
```

Install from source remains supported: [`scripts/install-cli.sh`](../scripts/install-cli.sh).

### Regenerating cargo-dist CI

After changing [`dist-workspace.toml`](../dist-workspace.toml) or dist metadata in crate `Cargo.toml` files:

```bash
dist generate
```

After `dist generate`, re-apply the **Run Rust verify** step in the `plan` job of [`.github/workflows/release.yml`](../.github/workflows/release.yml) (see [CI.md](CI.md)).

## Extension (rex-vscode)

| Item | Detail |
|------|--------|
| Version | [`extensions/rex-vscode/package.json`](../extensions/rex-vscode/package.json) |
| Changelog | [`extensions/rex-vscode/CHANGELOG.md`](../extensions/rex-vscode/CHANGELOG.md) |
| Release PR bot | [release-please](https://github.com/googleapis/release-please) — [`.github/workflows/release-please-extension.yml`](../.github/workflows/release-please-extension.yml) |
| Tag | `rex-vscode-vX.Y.Z` |
| Publish | [`.github/workflows/extension-release.yml`](../.github/workflows/extension-release.yml) (VSIX, optional Open VSX / Marketplace) |

### Maintainer flow

1. Merge extension changes with conventional commits.
2. Merge the **release-please** Release PR (`release-extension` label) when ready.
3. release-please creates `rex-vscode-v*`; **Extension Release** builds and attaches the VSIX (and publishes when secrets are set).

Details: [EXTENSION_RELEASE.md](EXTENSION_RELEASE.md).

## When not to merge a Release PR

- **RC-*** Must criteria in [V1_0.md](V1_0.md) are not Met but the proposed version is `1.0.0`.
- Breaking `rex.v1` or sidecar API change without a versioned migration plan ([DEVELOPER_EXPERIENCE_GUIDE.md](DEVELOPER_EXPERIENCE_GUIDE.md) §5).
- Release PR includes unrelated changes (re-run bots after fixing `main`).

## CI dry-runs

```bash
# Extension package only (no publish)
gh workflow run "Extension Release" -f dry_run=true

# Release-plz / release-please run on push to main; use workflow_dispatch where enabled
gh workflow run "Release-plz"
gh workflow run "Release Please (extension)"
```

## Related

- [CI.md](CI.md) — PR checks and release workflow observability
- [CONTRIBUTING.md](../CONTRIBUTING.md) — commit format
- [V1_0.md](V1_0.md) — `1.0.0` tagging gate

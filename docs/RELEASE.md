# Release automation

REX uses **release-plz** for the Rust workspace (`v*` tags). [Conventional Commits](../CONTRIBUTING.md) feed changelog generation; a **Release PR** on `main` precedes tagging.

## Core (Rust workspace)

| Item | Detail |
|------|--------|
| Version | `[workspace.package].version` in root [`Cargo.toml`](../Cargo.toml) |
| Changelog | [`CHANGELOG.md`](../CHANGELOG.md) |
| Release PR bot | [release-plz](https://release-plz.dev/) — workflow [`.github/workflows/release-plz.yml`](../.github/workflows/release-plz.yml), config [`release-plz.toml`](../release-plz.toml) |
| Tag | `vX.Y.Z` (for example `v0.1.1`) |
| Binaries | [cargo-dist](https://axodotdev.github.io/cargo-dist/) — workflow [`.github/workflows/release.yml`](../.github/workflows/release.yml) |
| Artifacts | **`rex`** (primary), `rex-cli`, `rex-daemon`, `rex-sidecar-stub` per target triple (`rex-cli` / `rex-daemon` are compatibility shims) |

### Maintainer flow

1. Merge feature PRs to `main` with conventional commit messages (or squash titles that match [CONTRIBUTING.md](../CONTRIBUTING.md)).
2. Wait for **Release-plz** to open or update a Release PR (`release-core` label). Review version bump and [`CHANGELOG.md`](../CHANGELOG.md).
3. **Tagging `1.0.0`:** every Must **RC-*** in [V1_0.md](V1_0.md) is **Met** (**RC-01–RC-10** and observability **RC-LF1**); merge the release-plz Release PR to create the **`v1.0.0`** tag (workspace version is already **`1.0.0`** in tree; **git tag blocked** until those rows close).
4. Merge the Release PR. release-plz creates the `v*` tag and a GitHub Release (notes only).
5. **Core Release** workflow runs on the tag: runs [`scripts/ci/run_rust_verify.sh`](../scripts/ci/run_rust_verify.sh), builds archives with cargo-dist, uploads assets to the same GitHub Release.

### Local binary dry-run

```bash
cargo install cargo-dist --locked # once
./scripts/release/build_core_artifacts.sh v1.0.0
```

Install from source remains supported: [`scripts/install-cli.sh`](../scripts/install-cli.sh).

### Regenerating cargo-dist CI

After changing [`dist-workspace.toml`](../dist-workspace.toml) or dist metadata in crate `Cargo.toml` files:

```bash
dist generate
```

After `dist generate`, re-apply the **Run Rust verify** and **`sidecar-verify`** jobs in [`.github/workflows/release.yml`](../.github/workflows/release.yml) (see [CI.md](CI.md)).

## When not to merge a Release PR

- Proposed **major** or breaking `rex.v1` / sidecar API change without a versioned migration plan (Must **RC-*** are Met for the current `1.0.0` line).
- Breaking `rex.v1` or sidecar API change without a versioned migration plan ([DEVELOPER_EXPERIENCE_GUIDE.md](DEVELOPER_EXPERIENCE_GUIDE.md) §5).
- Release PR includes unrelated changes (re-run bots after fixing `main`).

## CI dry-runs

```bash
# Release-plz runs on push to main; use workflow_dispatch where enabled
gh workflow run "Release-plz"
```

## Related

- [CI.md](CI.md) — PR checks and release workflow observability
- [CONTRIBUTING.md](../CONTRIBUTING.md) — commit format
- [V1_0.md](V1_0.md) — `1.0.0` tagging gate

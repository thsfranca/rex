# Contributing to REX

## Commit messages

REX uses [Conventional Commits](https://www.conventionalcommits.org/) so release automation can propose SemVer bumps and changelogs.

### Format

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

### Types

| Type | SemVer | Changelog section |
|------|--------|-------------------|
| `feat` | minor (patch in `0.x`) | Added |
| `fix` | patch | Fixed |
| `docs` | none | Documentation |
| `chore` | none | Other |
| `refactor` | none | Changed |
| `test` | none | Other |
| `ci` | none | Other |

### Breaking changes

Use one of:

- `feat!:` or `fix!:` in the subject, or
- a footer line: `BREAKING CHANGE: <description>`

Breaking changes require a **major** bump now that the workspace is at **`1.0.0`** in [Cargo.toml](Cargo.toml).

### Scopes (optional)

Examples: `feat(cli):`, `fix(daemon):`, `docs:`. Scopes are optional but help changelog grouping.

### Pull request titles

PR titles must follow the same convention when using squash merge (the title becomes the commit on `main`). CI requires conventional PR titles before merge.

## Releases

See [docs/RELEASE.md](docs/RELEASE.md) for core (`v*`) release automation.

## Development

Local quality gates: [docs/DEVELOPER_EXPERIENCE_GUIDE.md](docs/DEVELOPER_EXPERIENCE_GUIDE.md) §3 and [docs/CI.md](docs/CI.md).

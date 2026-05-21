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

Breaking changes require a **major** bump once the workspace is at `1.0.0`. While the workspace is `0.x`, breaking changes still map to minor per SemVer pre-1.0 rules.

### Scopes (optional)

Examples: `feat(cli):`, `fix(daemon):`, `feat(extension):`. Scopes are optional but help changelog grouping.

### Pull request titles

PR titles should follow the same convention when using squash merge (the title becomes the commit on `main`). CI may warn on non-conventional PR titles.

## Releases

See [docs/RELEASE.md](docs/RELEASE.md) for core (`v*`) and extension (`rex-vscode-v*`) release planes.

## Development

Local quality gates: [docs/DEVELOPER_EXPERIENCE_GUIDE.md](docs/DEVELOPER_EXPERIENCE_GUIDE.md) §3 and [docs/CI.md](docs/CI.md).

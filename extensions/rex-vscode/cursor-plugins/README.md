# Cursor plugins (bundled)

This directory is registered with `vscode.cursor.plugins.registerPath` when the extension activates inside Cursor. Plugin manifests shipped here become available to Cursor alongside the extension.

PR 1 ships this directory empty by design so the registration path is wired before any plugin content lands. Add plugin manifests in follow-up work (tracked in `docs/EXTENSION_ROADMAP.md`).

In plain VS Code, this directory is ignored because `vscode.cursor` is absent.

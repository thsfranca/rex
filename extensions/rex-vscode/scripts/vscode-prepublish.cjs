#!/usr/bin/env node
/**
 * vscode:prepublish — vsce invokes this before pack/publish.
 * Set SKIP_VSCE_PREPUBLISH=1 when dist is already built (CI after npm run build).
 */
if (process.env.SKIP_VSCE_PREPUBLISH === "1") {
  process.exit(0);
}

const { execSync } = require("node:child_process");
execSync("npm run build", { stdio: "inherit" });

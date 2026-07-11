"use strict";

const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");

const DEFAULT_SOCKET = "/tmp/rex.sock";

function readJson(filePath) {
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch {
    return null;
  }
}

function workspaceSocketName(cwd) {
  const hash = crypto.createHash("sha256").update(cwd).digest("hex").slice(0, 16);
  return `ws-${hash}.sock`;
}

/**
 * Resolve daemon UDS path (mirrors rex-cli / rex-config).
 */
function resolveDaemonSocket() {
  const rexRoot = process.env.REX_ROOT;
  if (rexRoot) {
    const cfg = readJson(path.join(rexRoot, "config.json"));
    const daemon = cfg?.daemon ?? {};
    if (typeof daemon.socket === "string" && daemon.socket.length > 0) {
      return daemon.socket;
    }
    if (daemon.socket_scope === "per_workspace") {
      const cwd = process.cwd();
      return path.join(rexRoot, "sockets", workspaceSocketName(cwd));
    }
  }
  return DEFAULT_SOCKET;
}

function resolveRexRoot() {
  return process.env.REX_ROOT || null;
}

module.exports = {
  resolveDaemonSocket,
  resolveRexRoot,
  DEFAULT_SOCKET,
};

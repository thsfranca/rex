"use strict";

const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawn } = require("node:child_process");
const { createRexClient, unary } = require("./grpcClient.cjs");
const { resolveDaemonSocket, resolveRexRoot } = require("./resolveSocket.cjs");
const { toSystemStatus } = require("./streamMap.cjs");

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

function findRexBinary() {
  for (const key of ["CARGO_BIN_EXE_rex", "REX_BIN"]) {
    const p = process.env[key];
    if (p && fs.existsSync(p)) return p;
  }
  const candidates = [
    path.resolve(__dirname, "../../../../target/debug/rex"),
    path.resolve(__dirname, "../../../../target/release/rex"),
  ];
  for (const c of candidates) {
    if (fs.existsSync(c)) return c;
  }
  return null;
}

/**
 * Spawn cwd must not walk into ~/.rex as a project overlay when REX_ROOT is a
 * fixture under $HOME (config merge walks cwd upward for .rex/config.json).
 */
function daemonSpawnCwd() {
  return os.tmpdir();
}

async function probeStatus() {
  const client = createRexClient();
  try {
    const res = await unary(client, "GetSystemStatus", {});
    return toSystemStatus(res);
  } finally {
    client.close?.();
  }
}

function spawnDaemon() {
  const rexBin = findRexBinary();
  if (!rexBin) {
    throw new Error(
      "Could not find rex binary to autostart daemon; set REX_BIN or build with cargo build -p rex",
    );
  }
  const socket = resolveDaemonSocket();
  const logPath = resolveRexRoot()
    ? path.join(resolveRexRoot(), "daemon.log")
    : path.join(process.cwd(), "daemon.log");
  const out = fs.openSync(logPath, "a");
  const child = spawn(rexBin, ["__rex_internal_daemon"], {
    detached: true,
    stdio: ["ignore", out, out],
    cwd: daemonSpawnCwd(),
    env: { ...process.env },
  });
  child.unref();
  return { pid: child.pid, socket };
}

/**
 * Ensure daemon answers GetSystemStatus; autostart if needed.
 */
async function ensureDaemonReady(timeoutMs = 60_000) {
  try {
    return await probeStatus();
  } catch {
    // fall through to autostart
  }

  spawnDaemon();
  const deadline = Date.now() + timeoutMs;
  let lastErr = null;
  while (Date.now() < deadline) {
    await sleep(250);
    try {
      return await probeStatus();
    } catch (err) {
      lastErr = err;
    }
  }
  throw new Error(
    `Daemon did not become ready at ${resolveDaemonSocket()}: ${lastErr?.message || lastErr}`,
  );
}

async function getSystemStatus() {
  return probeStatus();
}

async function probeLifecycle() {
  try {
    const status = await probeStatus();
    return { kind: "ready", workspaceRoot: status.workspaceRoot };
  } catch (err) {
    return {
      kind: "unavailable",
      message: err?.message || String(err),
    };
  }
}

module.exports = {
  ensureDaemonReady,
  getSystemStatus,
  probeLifecycle,
  findRexBinary,
};

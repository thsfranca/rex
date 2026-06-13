#!/usr/bin/env node
/**
 * Static server + esbuild watch for the browser Design Mode harness.
 * Serves extensions/rex-vscode/ at http://127.0.0.1:3456/
 */

import { spawn } from "node:child_process";
import { createReadStream, existsSync, statSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";

const PORT = 3456;
const ROOT = join(fileURLToPath(new URL(".", import.meta.url)), "..");

const MIME = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".map": "application/json; charset=utf-8",
  ".svg": "image/svg+xml",
};

function spawnWatch(label, script) {
  const child = spawn(process.execPath, [script, "--watch"], {
    cwd: ROOT,
    stdio: "inherit",
  });
  child.on("exit", (code) => {
    if (code !== 0 && code !== null) {
      console.error(`[rex dev] ${label} exited with code ${code}`);
      process.exit(code);
    }
  });
  return child;
}

function runBuild(script) {
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [script], {
      cwd: ROOT,
      stdio: "inherit",
    });
    child.on("error", reject);
    child.on("exit", (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`${script} failed with code ${code}`));
      }
    });
  });
}

function safePath(urlPath) {
  const decoded = decodeURIComponent(urlPath.split("?")[0] ?? "/");
  const relative = decoded === "/" ? "dev/index.html" : decoded.replace(/^\//, "");
  const absolute = normalize(join(ROOT, relative));
  if (!absolute.startsWith(ROOT)) {
    return null;
  }
  return absolute;
}

function createStaticServer() {
  return createServer((req, res) => {
    const target = safePath(req.url ?? "/");
    if (target === null || !existsSync(target) || statSync(target).isDirectory()) {
      res.writeHead(404, { "Content-Type": "text/plain; charset=utf-8" });
      res.end("Not found");
      return;
    }

    const type = MIME[extname(target)] ?? "application/octet-stream";
    res.writeHead(200, { "Content-Type": type });
    createReadStream(target).pipe(res);
  });
}

async function main() {
  await runBuild("esbuild.webview.mjs");
  await runBuild("esbuild.dev.mjs");

  spawnWatch("webview", join(ROOT, "esbuild.webview.mjs"));
  spawnWatch("harness mock", join(ROOT, "esbuild.dev.mjs"));

  const server = createStaticServer();
  server.listen(PORT, "127.0.0.1", () => {
    console.log("");
    console.log(`[rex dev] Design Mode harness: http://127.0.0.1:${PORT}/`);
    console.log("[rex dev] Watching dist/webview.js and dev/mock-host.js");
    console.log("");
  });
}

main().catch((error) => {
  console.error("[rex dev] failed to start:", error);
  process.exit(1);
});

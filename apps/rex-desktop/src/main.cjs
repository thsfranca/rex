"use strict";

const fs = require("node:fs");
const path = require("node:path");
const { app, BrowserWindow } = require("electron");
const { registerDaemonIpc } = require("./ipc/registerHandlers.cjs");

const APP_ROOT = path.join(__dirname, "..");
const WEB_DIST = path.resolve(APP_ROOT, "..", "rex-web", "dist", "index.html");

let mainWindow = null;
let stopLifecycle = null;

function parseArgs(argv) {
  const flags = new Set(argv.slice(2));
  return {
    // Forces Electric Alive ambient draw in the real apps/rex-web UI (CI compositor gate).
    compositorProof:
      flags.has("--compositor-proof") ||
      process.env.REX_COMPOSITOR_PROOF === "1",
    debug: flags.has("--debug") || process.env.REX_DESKTOP_DEBUG === "1",
  };
}

function createWindow({ debug }) {
  const win = new BrowserWindow({
    width: 1200,
    height: 800,
    minWidth: 640,
    minHeight: 480,
    show: true,
    title: "Rex",
    backgroundColor: "#0a0c10",
    webPreferences: {
      preload: path.join(__dirname, "preload.cjs"),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
  });

  if (fs.existsSync(WEB_DIST)) {
    // Do not append ?query to file:// — Chromium leaves #root empty for the Vite bundle.
    // Compositor proof is signaled via rex:launchOptions IPC instead.
    win.loadFile(WEB_DIST);
  } else {
    win.loadURL(
      "data:text/html," +
        encodeURIComponent(
          "<h1>Rex</h1><p>Missing apps/rex-web/dist. Run: cd apps/rex-web && npm run build</p>",
        ),
    );
  }

  if (debug) {
    win.webContents.openDevTools({ mode: "detach" });
  }
  return win;
}

app.whenReady().then(() => {
  const opts = parseArgs(process.argv);
  stopLifecycle = registerDaemonIpc(
    () => mainWindow,
    { debug: opts.debug, compositorProof: opts.compositorProof },
  );
  mainWindow = createWindow(opts);

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      mainWindow = createWindow(opts);
    }
  });
});

app.on("window-all-closed", () => {
  if (typeof stopLifecycle === "function") {
    stopLifecycle();
    stopLifecycle = null;
  }
  if (process.platform !== "darwin") {
    app.quit();
  }
});

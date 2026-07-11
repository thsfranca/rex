"use strict";

const fs = require("node:fs");
const path = require("node:path");
const { app, BrowserWindow } = require("electron");
const { registerDaemonIpc } = require("./ipc/registerHandlers.cjs");

const APP_ROOT = path.join(__dirname, "..");
const PROOF_DIR = path.join(APP_ROOT, "proof");
const WEB_DIST = path.resolve(APP_ROOT, "..", "rex-web", "dist", "index.html");

let mainWindow = null;
let stopLifecycle = null;

function parseArgs(argv) {
  const flags = new Set(argv.slice(2));
  return {
    proof: flags.has("--proof") || process.env.REX_ELECTRON_PROOF === "1",
    bury: flags.has("--bury") || process.env.REX_COMPOSITOR_PROOF_BURY === "1",
    debug: flags.has("--debug") || process.env.REX_DESKTOP_DEBUG === "1",
  };
}

function createWindow({ proof, bury, debug }) {
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

  if (proof) {
    const query = bury ? "?bury=1" : "";
    win.loadFile(path.join(PROOF_DIR, "index.html"), { search: query });
  } else if (fs.existsSync(WEB_DIST)) {
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
  if (!opts.proof) {
    stopLifecycle = registerDaemonIpc(
      () => mainWindow,
      { debug: opts.debug },
    );
  }
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

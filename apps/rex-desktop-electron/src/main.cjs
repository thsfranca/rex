"use strict";

const path = require("node:path");
const { app, BrowserWindow } = require("electron");

const PROOF_DIR = path.join(__dirname, "..", "proof");

function parseArgs(argv) {
  const flags = new Set(argv.slice(2));
  return {
    proof: flags.has("--proof") || process.env.REX_ELECTRON_PROOF === "1",
    bury: flags.has("--bury") || process.env.REX_COMPOSITOR_PROOF_BURY === "1",
  };
}

function createProofWindow(bury) {
  const win = new BrowserWindow({
    width: 960,
    height: 640,
    show: true,
    backgroundColor: "#0a0c10",
    webPreferences: {
      preload: path.join(__dirname, "preload.cjs"),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
  });

  const query = bury ? "?bury=1" : "";
  win.loadFile(path.join(PROOF_DIR, "index.html"), { search: query });
  return win;
}

app.whenReady().then(() => {
  const { proof, bury } = parseArgs(process.argv);
  if (!proof && process.env.REX_ELECTRON_PROOF !== "1") {
    // Scaffold default: still open the proof page until the real app ships (PR3).
  }
  createProofWindow(bury);

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createProofWindow(bury);
    }
  });
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});

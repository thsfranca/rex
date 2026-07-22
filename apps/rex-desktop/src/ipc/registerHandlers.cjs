"use strict";

const { ipcMain } = require("electron");
const {
  ensureDaemonReady,
  getSystemStatus,
  probeLifecycle,
} = require("../daemon/ensure.cjs");
const {
  submitPromptStream,
  fetchSessionEvents,
  respondToToolApproval,
} = require("../daemon/stream.cjs");

/**
 * @param {() => import('electron').BrowserWindow | null} getWindow
 * @param {{ debug: boolean, compositorProof?: boolean }} launchState
 */
function registerDaemonIpc(getWindow, launchState) {
  ipcMain.handle("rex:ensureDaemon", async () => ensureDaemonReady());
  ipcMain.handle("rex:getSystemStatus", async () => getSystemStatus());
  ipcMain.handle("rex:launchOptions", async () => ({
    debug: Boolean(launchState.debug),
    compositorProof: Boolean(launchState.compositorProof),
  }));
  ipcMain.handle("rex:listClosedSessions", async () => []);
  ipcMain.handle("rex:fetchSessionEvents", async (_e, args) =>
    fetchSessionEvents(args.harnessSessionId, args),
  );
  ipcMain.handle("rex:respondToToolApproval", async (_e, args) =>
    respondToToolApproval(
      args.approvalToken,
      args.approved,
      args.toolCallId,
      args.harnessSessionId,
    ),
  );

  ipcMain.handle("rex:submitPrompt", async (event, args) => {
    const { prompt, mode, channel } = args;
    const sender = event.sender;
    return submitPromptStream(prompt, mode, (evt) => {
      if (!sender.isDestroyed()) {
        sender.send(channel, evt);
      }
    });
  });

  const timer = setInterval(async () => {
    const win = getWindow();
    if (!win || win.isDestroyed()) return;
    const lifecycle = await probeLifecycle();
    win.webContents.send("daemon-lifecycle", lifecycle);
  }, 5000);

  return () => clearInterval(timer);
}

module.exports = { registerDaemonIpc };

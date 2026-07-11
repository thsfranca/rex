"use strict";

const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("rexDesktop", {
  host: "electron",
  shell: "rex-desktop",

  ensureDaemon: () => ipcRenderer.invoke("rex:ensureDaemon"),
  getLaunchOptions: () => ipcRenderer.invoke("rex:launchOptions"),
  getSystemStatus: () => ipcRenderer.invoke("rex:getSystemStatus"),
  listClosedSessions: () => ipcRenderer.invoke("rex:listClosedSessions"),
  fetchSessionEvents: (harnessSessionId, opts = {}) =>
    ipcRenderer.invoke("rex:fetchSessionEvents", {
      harnessSessionId,
      ...opts,
    }),
  respondToToolApproval: (
    approvalToken,
    approved,
    toolCallId,
    harnessSessionId,
  ) =>
    ipcRenderer.invoke("rex:respondToToolApproval", {
      approvalToken,
      approved,
      toolCallId,
      harnessSessionId,
    }),

  submitPrompt: (prompt, mode, onEvent) => {
    const channel = `rex:stream:${Date.now()}-${Math.random().toString(16).slice(2)}`;
    const handler = (_event, evt) => {
      onEvent(evt);
    };
    ipcRenderer.on(channel, handler);
    return ipcRenderer
      .invoke("rex:submitPrompt", { prompt, mode, channel })
      .finally(() => {
        ipcRenderer.removeListener(channel, handler);
      });
  },

  subscribeDaemonLifecycle: (handler) => {
    const h = (_e, payload) => handler(payload);
    ipcRenderer.on("daemon-lifecycle", h);
    return () => ipcRenderer.removeListener("daemon-lifecycle", h);
  },

  subscribeMenuAction: (handler) => {
    const h = (_e, action) => handler(action);
    ipcRenderer.on("menu-action", h);
    return () => ipcRenderer.removeListener("menu-action", h);
  },
});

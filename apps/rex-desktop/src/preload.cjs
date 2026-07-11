"use strict";

const { contextBridge } = require("electron");

// Product IPC (daemon UDS bridge) lands in the next slice. Host identity only for now.
contextBridge.exposeInMainWorld("rexDesktop", {
  host: "electron",
  shell: "rex-desktop",
});

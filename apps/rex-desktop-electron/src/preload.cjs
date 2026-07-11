"use strict";

const { contextBridge } = require("electron");

contextBridge.exposeInMainWorld("rexDesktop", {
  host: "electron",
  shell: "rex-desktop-electron",
});

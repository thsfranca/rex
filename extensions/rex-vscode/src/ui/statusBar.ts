import * as vscode from "vscode";

import type { DaemonLifecycleState } from "../runtime/daemonLifecycle";

const COMMAND_ID = "rex.showStatus";

export interface StatusBar {
  readonly item: vscode.StatusBarItem;
  update(state: DaemonLifecycleState): void;
  dispose(): void;
}

export function createStatusBar(): StatusBar {
  const item = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Left,
    100,
  );
  item.command = COMMAND_ID;
  item.name = "REX";
  renderState(item, { kind: "unavailable", reason: "not probed yet" });
  item.show();
  return {
    item,
    update(state) {
      renderState(item, state);
    },
    dispose() {
      item.dispose();
    },
  };
}

function renderState(item: vscode.StatusBarItem, state: DaemonLifecycleState): void {
  switch (state.kind) {
    case "ready": {
      item.text = "$(zap) REX ready";
      item.tooltip = `rex-daemon ${state.status.daemonVersion} (uptime ${state.status.uptimeSeconds}s)`;
      item.backgroundColor = undefined;
      return;
    }
    case "starting": {
      item.text = "$(sync~spin) REX starting";
      item.tooltip = "Waiting for rex-daemon to become ready.";
      item.backgroundColor = new vscode.ThemeColor("statusBarItem.warningBackground");
      return;
    }
    case "unavailable": {
      item.text = "$(circle-slash) REX unavailable";
      item.tooltip = `rex-daemon unavailable: ${state.reason}`;
      item.backgroundColor = new vscode.ThemeColor("statusBarItem.errorBackground");
      return;
    }
  }
}

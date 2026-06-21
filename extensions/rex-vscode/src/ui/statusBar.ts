import * as vscode from "vscode";

import { elideForTooltip } from "../runtime/cappedString";
import type { DaemonLifecycleState } from "../runtime/daemonLifecycle";

const COMMAND_ID = "rex.showStatus";
/** Max characters from `unavailable.reason` in the status tooltip (stack traces, spawn output). */
const UNAVAILABLE_TOOLTIP_MAX_CHARS = 800;

export interface StatusBar {
  readonly item: vscode.StatusBarItem;
  update(state: DaemonLifecycleState): void;
  setStreamingActivity(hint?: string): void;
  dispose(): void;
}

export function createStatusBar(): StatusBar {
  const item = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Left,
    100,
  );
  item.command = COMMAND_ID;
  item.name = "REX";
  let daemonState: DaemonLifecycleState = {
    kind: "unavailable",
    reason: "not probed yet",
  };
  let streamingHint: string | undefined;
  renderState(item, daemonState, streamingHint);
  item.show();
  return {
    item,
    update(state) {
      daemonState = state;
      renderState(item, daemonState, streamingHint);
    },
    setStreamingActivity(hint) {
      streamingHint = hint;
      renderState(item, daemonState, streamingHint);
    },
    dispose() {
      item.dispose();
    },
  };
}

function renderState(
  item: vscode.StatusBarItem,
  state: DaemonLifecycleState,
  streamingHint?: string,
): void {
  switch (state.kind) {
    case "ready": {
      if (streamingHint !== undefined && streamingHint.length > 0) {
        item.text = "$(sync~spin) REX running";
        item.tooltip = `rex: ${streamingHint}`;
        item.backgroundColor = new vscode.ThemeColor("statusBarItem.prominentBackground");
        return;
      }
      item.text = "$(zap) REX ready";
      item.tooltip = `rex daemon ${state.status.daemonVersion} (uptime ${state.status.uptimeSeconds}s)`;
      item.backgroundColor = undefined;
      return;
    }
    case "idle": {
      if (streamingHint !== undefined && streamingHint.length > 0) {
        item.text = "$(sync~spin) REX running";
        item.tooltip = `rex: ${streamingHint}`;
        item.backgroundColor = new vscode.ThemeColor("statusBarItem.prominentBackground");
        return;
      }
      const shutdownHint =
        state.status.secondsUntilShutdown > 0
          ? `; shutdown in ${state.status.secondsUntilShutdown}s without activity`
          : "";
      item.text = "$(watch) REX idle";
      item.tooltip = `rex daemon idle ${state.status.idleSeconds}s${shutdownHint}`;
      item.backgroundColor = undefined;
      return;
    }
    case "starting": {
      item.text = "$(sync~spin) REX starting";
      item.tooltip = "Waiting for rex daemon to become ready.";
      item.backgroundColor = new vscode.ThemeColor("statusBarItem.warningBackground");
      return;
    }
    case "unavailable": {
      item.text = "$(circle-slash) REX unavailable";
      item.tooltip = `rex daemon unavailable: ${elideForTooltip(state.reason, UNAVAILABLE_TOOLTIP_MAX_CHARS)}`;
      item.backgroundColor = new vscode.ThemeColor("statusBarItem.errorBackground");
      return;
    }
  }
}

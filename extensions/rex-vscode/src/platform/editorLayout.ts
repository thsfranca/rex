import * as vscode from "vscode";

import type { ChatLocationSetting, EditorChatColumnSetting } from "../config/settings";

export type ChatSidebarPlacement = "left" | "right" | "none";

/** VS Code 1.106+ exposes `viewsContainers.secondarySidebar`. */
export function supportsSecondarySidebar(): boolean {
  const parts = vscode.version.split(".");
  const major = Number(parts[0]);
  const minor = Number(parts[1]);
  if (Number.isNaN(major) || Number.isNaN(minor)) {
    return false;
  }
  return major > 1 || (major === 1 && minor >= 106);
}

export function resolveChatSidebar(
  location: ChatLocationSetting,
  secondarySidebarSupported: boolean,
): ChatSidebarPlacement {
  switch (location) {
    case "left":
      return "left";
    case "editor":
      return "none";
    case "right":
      return secondarySidebarSupported ? "right" : "left";
    case "auto":
    default:
      return secondarySidebarSupported ? "right" : "left";
  }
}

export function resolveEditorViewColumn(column: EditorChatColumnSetting): vscode.ViewColumn {
  switch (column) {
    case "active":
      return vscode.ViewColumn.Active;
    case "one":
      return vscode.ViewColumn.One;
    case "two":
      return vscode.ViewColumn.Two;
    case "three":
      return vscode.ViewColumn.Three;
    case "beside":
    default:
      return vscode.ViewColumn.Beside;
  }
}

/** Updates `when` clauses for sidebar view containers (`rex.chatSidebar`). */
export async function applyChatLayoutContext(location: ChatLocationSetting): Promise<void> {
  const sidebar = resolveChatSidebar(location, supportsSecondarySidebar());
  await vscode.commands.executeCommand("setContext", "rex.chatSidebar", sidebar);
}

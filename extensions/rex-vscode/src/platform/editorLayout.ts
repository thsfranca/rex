import * as vscode from "vscode";

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

/** When true, chat lives in the activity bar (older hosts). */
export async function configureChatLayoutContext(): Promise<void> {
  await vscode.commands.executeCommand(
    "setContext",
    "rex.useActivityBarChat",
    !supportsSecondarySidebar(),
  );
}

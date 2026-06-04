import * as vscode from "vscode";

import { buildWebviewHtml } from "./webviewHtml";

export const EDITOR_CHAT_PANEL_TYPE = "rex.editorChat";

let activePanel: vscode.WebviewPanel | undefined;

export function openEditorChatPanel(
  context: vscode.ExtensionContext,
  onMessage: (message: unknown) => void | Promise<void>,
): void {
  if (activePanel !== undefined) {
    activePanel.reveal(vscode.ViewColumn.Beside, true);
    return;
  }

  const panel = vscode.window.createWebviewPanel(
    EDITOR_CHAT_PANEL_TYPE,
    "REX Chat",
    vscode.ViewColumn.Beside,
    { enableScripts: true, retainContextWhenHidden: true },
  );
  activePanel = panel;
  panel.webview.options = {
    enableScripts: true,
    localResourceRoots: [vscode.Uri.joinPath(context.extensionUri, "dist")],
  };
  panel.webview.html = buildWebviewHtml(panel.webview, context.extensionUri);
  panel.webview.onDidReceiveMessage((raw) => {
    void onMessage(raw);
  });
  panel.onDidDispose(() => {
    activePanel = undefined;
  });
}

export function postToEditorPanel(message: unknown): void {
  void activePanel?.webview.postMessage(message);
}

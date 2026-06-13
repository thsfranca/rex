import * as vscode from "vscode";

import type { RexSettings } from "../config/settings";
import { resolveChatSidebar, resolveEditorViewColumn, supportsSecondarySidebar } from "../platform/editorLayout";

import { CHAT_VIEW_ID, CHAT_VIEW_SECONDARY_ID } from "./chatPanel";
import { openEditorChatPanel } from "./editorChatPanel";

export async function focusRexChat(
  context: vscode.ExtensionContext,
  settings: RexSettings,
  onEditorMessage: (message: unknown) => void | Promise<void>,
): Promise<void> {
  if (settings.chatLocation === "editor") {
    openEditorChatPanel(context, onEditorMessage, resolveEditorViewColumn(settings.editorChatColumn));
    return;
  }

  const sidebar = resolveChatSidebar(settings.chatLocation, supportsSecondarySidebar());
  const viewId = sidebar === "right" ? CHAT_VIEW_SECONDARY_ID : CHAT_VIEW_ID;
  await vscode.commands.executeCommand(`${viewId}.focus`);
}

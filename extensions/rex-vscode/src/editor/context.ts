import * as vscode from "vscode";

import type { PromptContextSnapshot } from "../shared/messages";

/**
 * Build a context snapshot from the currently active editor. Returns
 * `undefined` when there is no active editor so callers can render the
 * prompt without attaching editor context.
 */
export function snapshotActiveEditor(
  editor: vscode.TextEditor | undefined = vscode.window.activeTextEditor,
): PromptContextSnapshot | undefined {
  if (editor === undefined) {
    return undefined;
  }
  const document = editor.document;
  const selection = editor.selection;
  const selectionText = document.getText(selection);
  const hasSelection = !selection.isEmpty && selectionText.length > 0;
  return {
    filePath: document.uri.fsPath,
    languageId: document.languageId,
    selectionText: hasSelection ? selectionText : undefined,
    selectionRange: hasSelection
      ? {
          startLine: selection.start.line,
          startCharacter: selection.start.character,
          endLine: selection.end.line,
          endCharacter: selection.end.character,
        }
      : undefined,
  };
}

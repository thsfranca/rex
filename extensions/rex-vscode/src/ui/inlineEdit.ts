import * as vscode from "vscode";

import type { ChatPanelProvider } from "./chatPanel";
import type { RexProposalProvider } from "../editor/virtualDocs";
import type { CliBridgeOptions } from "../runtime/cliBridge";
import { streamComplete } from "../runtime/streamClient";
import { applyEditToActiveFile } from "./applyEdit";

export interface InlineEditDependencies {
  readonly getCliOptions: () => CliBridgeOptions;
  readonly getModelId: () => string;
  readonly getProposalProvider: () => RexProposalProvider;
  readonly log: (message: string) => void;
  readonly chatPanel: ChatPanelProvider;
}

export async function runInlineEditOnSelection(deps: InlineEditDependencies): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (editor === undefined || editor.selection.isEmpty) {
    void vscode.window.showWarningMessage("Select code to edit inline with REX.");
    return;
  }
  const instruction = await vscode.window.showInputBox({
    prompt: "Describe the change for the selected code",
    placeHolder: "Refactor for clarity…",
  });
  if (instruction === undefined || instruction.trim().length === 0) {
    return;
  }
  const selectionText = editor.document.getText(editor.selection);
  const languageId = editor.document.languageId;
  const prompt = `${instruction.trim()}\n\nApply only to this selection:\n\`\`\`${languageId}\n${selectionText}\n\`\`\``;

  const id = `inline-${Date.now()}`;
  const controller = new AbortController();
  let buffer = "";
  const configuredModel = deps.getModelId().trim();
  try {
    for await (const event of streamComplete(deps.getCliOptions(), {
      prompt,
      mode: "ask",
      model: configuredModel.length > 0 ? configuredModel : undefined,
      signal: controller.signal,
    })) {
      if (event.kind === "chunk") {
        buffer += event.text;
        continue;
      }
      if (event.kind === "done") {
        break;
      }
      void vscode.window.showErrorMessage(event.message);
      return;
    }
  } catch (error) {
    void vscode.window.showErrorMessage(error instanceof Error ? error.message : String(error));
    return;
  }

  const codeMatch = /```[\w-]*\n([\s\S]*?)```/.exec(buffer);
  const code = codeMatch?.[1]?.trim() ?? buffer.trim();
  if (code.length === 0) {
    void vscode.window.showWarningMessage("REX returned no editable code.");
    return;
  }

  const result = await applyEditToActiveFile(
    {
      id,
      code,
      language: languageId,
      granularity: "selection",
    },
    { provider: deps.getProposalProvider(), log: deps.log },
  );
  if (result.outcome === "applied") {
    void vscode.window.showInformationMessage("REX applied inline edit.");
  } else if (result.outcome === "rejected") {
    void vscode.window.showInformationMessage("Inline edit rejected.");
  }
}

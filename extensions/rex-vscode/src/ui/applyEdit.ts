import * as vscode from "vscode";

import type { ApplyGranularity, ApplyResultPayload } from "../shared/messages";
import type { RexProposalProvider } from "../editor/virtualDocs";

export interface ApplyEditRequest {
  readonly id: string;
  readonly code: string;
  readonly language: string;
  readonly granularity: ApplyGranularity;
}

export interface ApplyEditDependencies {
  readonly provider: RexProposalProvider;
  readonly log: (message: string) => void;
}

/**
 * Drive the Apply-to-file flow:
 *
 * 1. Capture the active editor + intended target range.
 * 2. Register the proposed content in the virtual-doc provider.
 * 3. Open VS Code's native diff view (left = current, right = proposal).
 * 4. Ask the user to Accept or Reject; Accept writes via `WorkspaceEdit`.
 *
 * Any failure is surfaced to the caller as an `ApplyResultPayload` so the
 * chat UI can render a specific outcome (applied / rejected / cancelled /
 * error) per code block.
 */
export async function applyEditToActiveFile(
  request: ApplyEditRequest,
  deps: ApplyEditDependencies,
): Promise<ApplyResultPayload> {
  const editor = vscode.window.activeTextEditor;
  if (editor === undefined) {
    deps.log(`[apply] no active editor for proposal ${request.id}`);
    return {
      outcome: "error",
      detail: "Open the target file in the editor, then retry Apply.",
    };
  }

  const document = editor.document;
  const targetRange = resolveTargetRange(document, editor.selection, request.granularity);
  const proposedContent = buildProposalContent(document, targetRange, request);
  const proposalUri = deps.provider.register(request.id, proposedContent);
  deps.provider.update(request.id, proposedContent);

  const baseTitle = vscode.workspace.asRelativePath(document.uri);
  const diffTitle = `${baseTitle} ↔ REX proposal`;

  try {
    await vscode.commands.executeCommand(
      "vscode.diff",
      document.uri,
      proposalUri,
      diffTitle,
      { preview: true, preserveFocus: false },
    );
  } catch (error) {
    deps.log(`[apply] diff open failed: ${stringifyError(error)}`);
    deps.provider.delete(request.id);
    return { outcome: "error", detail: "Failed to open diff view." };
  }

  const pick = await vscode.window.showInformationMessage(
    `Apply REX proposal to ${baseTitle}?`,
    { modal: false },
    "Apply",
    "Reject",
  );
  deps.provider.delete(request.id);

  if (pick === undefined) {
    return { outcome: "cancelled" };
  }
  if (pick === "Reject") {
    return { outcome: "rejected" };
  }

  const edit = new vscode.WorkspaceEdit();
  edit.replace(document.uri, targetRange, request.code);
  const ok = await vscode.workspace.applyEdit(edit);
  if (!ok) {
    deps.log(`[apply] WorkspaceEdit rejected for ${request.id}`);
    return { outcome: "error", detail: "VS Code refused the edit." };
  }
  return { outcome: "applied", detail: baseTitle };
}

function resolveTargetRange(
  document: vscode.TextDocument,
  selection: vscode.Selection,
  granularity: ApplyGranularity,
): vscode.Range {
  if (granularity === "selection" && !selection.isEmpty) {
    return new vscode.Range(selection.start, selection.end);
  }
  const lastLine = document.lineCount === 0 ? 0 : document.lineCount - 1;
  const end =
    document.lineCount === 0
      ? new vscode.Position(0, 0)
      : document.lineAt(lastLine).range.end;
  return new vscode.Range(new vscode.Position(0, 0), end);
}

function buildProposalContent(
  document: vscode.TextDocument,
  range: vscode.Range,
  request: ApplyEditRequest,
): string {
  if (document.lineCount === 0) {
    return request.code;
  }
  const fullEnd = document.lineAt(document.lineCount - 1).range.end;
  const replacesWholeFile =
    range.start.isEqual(new vscode.Position(0, 0)) && range.end.isEqual(fullEnd);
  if (replacesWholeFile) {
    return request.code;
  }
  const head = document.getText(new vscode.Range(new vscode.Position(0, 0), range.start));
  const tail = document.getText(new vscode.Range(range.end, fullEnd));
  return `${head}${request.code}${tail}`;
}

function stringifyError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

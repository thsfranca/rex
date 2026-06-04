import * as vscode from "vscode";

import type { ContextAttachment } from "../shared/messages";

interface ContextPickerChoice extends vscode.QuickPickItem {
  readonly contextKind: "file" | "symbol";
}

/** QuickPick-based context: workspace files and symbols in the active document. */
export async function pickContextAttachments(): Promise<ReadonlyArray<ContextAttachment>> {
  const pick = await vscode.window.showQuickPick<ContextPickerChoice>(
    [
      { label: "File in workspace…", contextKind: "file" },
      { label: "Symbol in active file…", contextKind: "symbol" },
    ],
    { placeHolder: "Add context to REX" },
  );
  if (pick === undefined) {
    return [];
  }
  if (pick.contextKind === "file") {
    return pickWorkspaceFile();
  }
  return pickDocumentSymbol();
}

export function formatAttachmentsForPrompt(
  attachments: ReadonlyArray<ContextAttachment>,
): string {
  if (attachments.length === 0) {
    return "";
  }
  const blocks = attachments.map(
    (item) => `[@${item.kind}:${item.label}]\n${item.text}`,
  );
  return `\n\nAttached context:\n${blocks.join("\n\n")}`;
}

async function pickWorkspaceFile(): Promise<ReadonlyArray<ContextAttachment>> {
  const uris = await vscode.window.showOpenDialog({
    canSelectMany: false,
    openLabel: "Attach file",
  });
  if (uris === undefined || uris.length === 0) {
    return [];
  }
  const uri = uris[0];
  let text = "";
  try {
    const doc = await vscode.workspace.openTextDocument(uri);
    text = doc.getText();
  } catch {
    text = "";
  }
  const relative = vscode.workspace.asRelativePath(uri, false);
  return [
    {
      id: `file-${uri.fsPath}`,
      kind: "file",
      label: relative,
      text: text.slice(0, 16_000),
    },
  ];
}

async function pickDocumentSymbol(): Promise<ReadonlyArray<ContextAttachment>> {
  const editor = vscode.window.activeTextEditor;
  if (editor === undefined) {
    void vscode.window.showWarningMessage("Open a file to pick a symbol.");
    return [];
  }
  const symbols = await vscode.commands.executeCommand<
    vscode.DocumentSymbol[] | undefined
  >("vscode.executeDocumentSymbolProvider", editor.document.uri);
  if (symbols === undefined || symbols.length === 0) {
    void vscode.window.showInformationMessage("No symbols found in the active file.");
    return [];
  }
  const flat = flattenSymbols(symbols);
  const pick = await vscode.window.showQuickPick(
    flat.map((symbol) => ({
      label: symbol.name,
      description: vscode.SymbolKind[symbol.kind],
      symbol,
    })),
    { placeHolder: "Pick a symbol" },
  );
  if (pick === undefined) {
    return [];
  }
  const range = pick.symbol.range;
  const text = editor.document.getText(range);
  return [
    {
      id: `symbol-${pick.symbol.name}-${range.start.line}`,
      kind: "symbol",
      label: pick.symbol.name,
      text,
    },
  ];
}

function flattenSymbols(symbols: ReadonlyArray<vscode.DocumentSymbol>): vscode.DocumentSymbol[] {
  const out: vscode.DocumentSymbol[] = [];
  for (const symbol of symbols) {
    out.push(symbol);
    if (symbol.children.length > 0) {
      out.push(...flattenSymbols(symbol.children));
    }
  }
  return out;
}

import * as vscode from "vscode";

import type { RexProposalProvider } from "../editor/virtualDocs";
import { applyEditToActiveFile } from "./applyEdit";

export interface FileProposal {
  readonly id: string;
  readonly filePath: string;
  readonly code: string;
  readonly language: string;
}

export async function reviewMultiFileProposals(
  proposals: ReadonlyArray<FileProposal>,
  provider: RexProposalProvider,
  log: (message: string) => void,
): Promise<void> {
  if (proposals.length === 0) {
    return;
  }
  if (proposals.length === 1) {
    const only = proposals[0];
    await applyEditToActiveFile(
      {
        id: only.id,
        code: only.code,
        language: only.language,
        granularity: "file",
      },
      { provider, log },
    );
    return;
  }

  const pick = await vscode.window.showQuickPick(
    proposals.map((proposal) => ({
      label: proposal.filePath,
      description: "Review proposed change",
      proposal,
    })),
    { placeHolder: "Select a file to review (multi-file batch)" },
  );
  if (pick === undefined) {
    return;
  }
  const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(pick.proposal.filePath));
  await vscode.window.showTextDocument(doc, { preview: false });
  await applyEditToActiveFile(
    {
      id: pick.proposal.id,
      code: pick.proposal.code,
      language: pick.proposal.language,
      granularity: "file",
    },
    { provider, log },
  );
}

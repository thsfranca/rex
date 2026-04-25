import * as vscode from "vscode";

export const REX_PROPOSAL_SCHEME = "rex-proposal";

/**
 * Serves proposed code changes as virtual read-only documents so VS Code's
 * native `vscode.diff` command can render them alongside the user's file
 * without touching disk.
 *
 * Contents are keyed by the `path` portion of the URI and owned by the host
 * (not persisted). The host must call `register` / `update` / `delete` to
 * manage the lifecycle of each proposal.
 */
export class RexProposalProvider implements vscode.TextDocumentContentProvider {
  private readonly store = new Map<string, string>();
  private readonly emitter = new vscode.EventEmitter<vscode.Uri>();

  readonly onDidChange = this.emitter.event;

  provideTextDocumentContent(uri: vscode.Uri): string {
    return this.store.get(uri.path) ?? "";
  }

  /**
   * Returns a URI for the given proposal id. Callers must invoke `update`
   * before opening the URI with `vscode.diff`.
   */
  register(id: string, content: string): vscode.Uri {
    this.store.set(normalize(id), content);
    return this.uriFor(id);
  }

  update(id: string, content: string): void {
    const key = normalize(id);
    this.store.set(key, content);
    this.emitter.fire(this.uriFor(id));
  }

  delete(id: string): void {
    this.store.delete(normalize(id));
  }

  uriFor(id: string): vscode.Uri {
    return vscode.Uri.from({
      scheme: REX_PROPOSAL_SCHEME,
      path: normalize(id),
    });
  }

  dispose(): void {
    this.store.clear();
    this.emitter.dispose();
  }
}

function normalize(id: string): string {
  return id.startsWith("/") ? id : `/${id}`;
}

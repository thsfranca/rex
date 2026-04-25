import * as crypto from "node:crypto";

import * as vscode from "vscode";

import { snapshotActiveEditor } from "../editor/context";
import { RexProposalProvider } from "../editor/virtualDocs";
import type { CliBridgeOptions } from "../runtime/cliBridge";
import type { DaemonLifecycleState } from "../runtime/daemonLifecycle";
import { classifyStreamError, classifyStreamErrorMessage } from "../runtime/errorTaxonomy";
import { resolveModePolicy } from "../runtime/modePolicy";
import { streamComplete } from "../runtime/streamClient";
import type {
  ApprovalDecisionPayload,
  ApprovalScope,
  ExtensionToWebview,
  InteractionMode,
  ModePolicy,
  PromptPrefillPayload,
  ThemeKind,
  WebviewToExtension,
} from "../shared/messages";

import { applyEditToActiveFile } from "./applyEdit";

export const CHAT_VIEW_ID = "rex.chatView";

export interface ChatPanelDependencies {
  readonly context: vscode.ExtensionContext;
  readonly getCliOptions: () => CliBridgeOptions;
  readonly getDaemonAutoStart: () => boolean;
  readonly ensureDaemonReady: (signal?: AbortSignal) => Promise<DaemonLifecycleState>;
  readonly getDaemonState: () => DaemonLifecycleState | undefined;
  readonly log: (message: string) => void;
}

type PendingStream = { readonly controller: AbortController };
type PendingApproval = {
  readonly resolve: (approved: boolean) => void;
  readonly scope: ApprovalScope;
};

/**
 * Webview-view provider that hosts the chat UI and brokers all messages
 * between the host-side runtime (stream client, apply flow, editor context)
 * and the React webview bundle.
 */
export class ChatPanelProvider implements vscode.WebviewViewProvider, vscode.Disposable {
  private view: vscode.WebviewView | undefined;
  private readonly proposalProvider = new RexProposalProvider();
  private readonly disposables: vscode.Disposable[] = [];
  private readonly pendingStreams = new Map<string, PendingStream>();
  private readonly pendingApprovals = new Map<string, PendingApproval>();
  private readonly pendingPrefills: PromptPrefillPayload[] = [];
  private mode: InteractionMode = "ask";

  constructor(private readonly deps: ChatPanelDependencies) {}

  register(): vscode.Disposable {
    const providerRegistration = vscode.window.registerWebviewViewProvider(
      CHAT_VIEW_ID,
      this,
      { webviewOptions: { retainContextWhenHidden: true } },
    );
    const docRegistration = vscode.workspace.registerTextDocumentContentProvider(
      "rex-proposal",
      this.proposalProvider,
    );
    const selectionListener = vscode.window.onDidChangeTextEditorSelection(() => {
      this.sendContextSnapshot();
    });
    const activeEditorListener = vscode.window.onDidChangeActiveTextEditor(() => {
      this.sendContextSnapshot();
    });
    const themeListener = vscode.window.onDidChangeActiveColorTheme((theme) => {
      this.postMessage({ type: "theme", payload: { kind: mapThemeKind(theme) } });
    });
    this.disposables.push(providerRegistration, docRegistration, selectionListener, activeEditorListener, themeListener);
    return new vscode.Disposable(() => this.dispose());
  }

  dispose(): void {
    for (const pending of this.pendingStreams.values()) {
      pending.controller.abort();
    }
    this.pendingStreams.clear();
    for (const pending of this.pendingApprovals.values()) {
      pending.resolve(false);
    }
    this.pendingApprovals.clear();
    this.pendingPrefills.length = 0;
    this.proposalProvider.dispose();
    for (const item of this.disposables) {
      item.dispose();
    }
    this.disposables.length = 0;
    this.view = undefined;
  }

  resolveWebviewView(view: vscode.WebviewView): void {
    this.view = view;
    const { webview } = view;
    webview.options = {
      enableScripts: true,
      localResourceRoots: [vscode.Uri.joinPath(this.deps.context.extensionUri, "dist")],
    };
    webview.html = renderWebviewHtml(webview, this.deps.context.extensionUri);

    const messageSub = webview.onDidReceiveMessage(async (raw: unknown) => {
      if (!isIncomingMessage(raw)) {
        return;
      }
      await this.handleWebviewMessage(raw);
    });
    const disposeSub = view.onDidDispose(() => {
      messageSub.dispose();
      disposeSub.dispose();
      this.view = undefined;
    });
  }

  broadcastDaemonState(state: DaemonLifecycleState | undefined): void {
    if (state === undefined) {
      return;
    }
    if (state.kind === "ready") {
      this.postMessage({
        type: "daemonState",
        payload: { state: "ready", detail: state.status.daemonVersion },
      });
      return;
    }
    if (state.kind === "starting") {
      this.postMessage({ type: "daemonState", payload: { state: "starting" } });
      return;
    }
    this.postMessage({
      type: "daemonState",
      payload: { state: "unavailable", detail: state.reason },
    });
  }

  prefillPrompt(payload: PromptPrefillPayload): void {
    if (this.view === undefined) {
      this.pendingPrefills.push(payload);
      return;
    }
    this.postMessage({ type: "prefillPrompt", payload });
    this.view.show?.(true);
  }

  clearChat(): void {
    this.postMessage({ type: "clearChat" });
  }

  private async handleWebviewMessage(message: WebviewToExtension): Promise<void> {
    switch (message.type) {
      case "ready":
        this.broadcastDaemonState(this.deps.getDaemonState());
        this.postMessage({ type: "modeState", payload: this.modePolicy() });
        this.postMessage({
          type: "theme",
          payload: { kind: mapThemeKind(vscode.window.activeColorTheme) },
        });
        this.sendContextSnapshot();
        this.flushPendingPrefills();
        return;
      case "submitPrompt":
        await this.handleSubmitPrompt(message);
        return;
      case "setMode":
        this.mode = message.mode;
        this.deps.log(`[chat] mode changed -> ${this.mode}`);
        this.postMessage({ type: "modeState", payload: this.modePolicy() });
        this.postMessage({
          type: "statusMessage",
          level: "info",
          text: `Mode set to ${this.mode.toUpperCase()}.`,
        });
        return;
      case "approvalDecision":
        this.resolveApproval(message.payload);
        return;
      case "cancelStream":
        this.cancelPendingStream(message.id);
        return;
      case "applyCodeBlock":
        await this.handleApplyCodeBlock(message);
        return;
      case "insertCodeBlock":
        await this.handleInsertCodeBlock(message);
        return;
      case "copyCodeBlock":
        await vscode.env.clipboard.writeText(message.code);
        this.postMessage({
          type: "statusMessage",
          level: "info",
          text: "Copied to clipboard.",
        });
        return;
      case "requestContextSnapshot":
        this.sendContextSnapshot();
        return;
      case "clearChatRequested":
        this.clearChat();
        return;
    }
  }

  private async handleSubmitPrompt(
    message: Extract<WebviewToExtension, { type: "submitPrompt" }>,
  ): Promise<void> {
    this.cancelPendingStream(message.id);
    const controller = new AbortController();
    this.pendingStreams.set(message.id, { controller });

    const fullPrompt = buildPromptWithContext(
      message.prompt,
      message.attachContext ? message.context : undefined,
    );

    this.postMessage({ type: "streamStarted", id: message.id });
    this.emitExecutionStep(message.id, "queued", `Request queued in ${this.mode.toUpperCase()} mode.`);

    try {
      if (this.modePolicy().requiresExecutionApproval) {
        const approved = await this.requestApproval(
          "execution",
          "Approve execution",
          `Run this request in ${this.mode.toUpperCase()} mode?`,
        );
        if (!approved) {
          this.emitExecutionStep(message.id, "blocked", "Execution blocked by user approval gate.");
          this.postMessage({
            type: "streamError",
            id: message.id,
            message: "Execution blocked: approval was not granted.",
            code: "cancelled",
            retryable: true,
          });
          return;
        }
      }

      if (this.deps.getDaemonAutoStart()) {
        const daemonState = await this.deps.ensureDaemonReady(controller.signal);
        if (daemonState.kind !== "ready") {
          const detail =
            daemonState.kind === "unavailable"
              ? daemonState.reason
              : "Daemon is still starting; try again in a moment.";
          this.postMessage({ type: "streamError", id: message.id, message: detail });
          return;
        }
      }

      this.emitExecutionStep(message.id, "running", "Execution started.");
      for await (const event of streamComplete(this.deps.getCliOptions(), {
        prompt: fullPrompt,
        signal: controller.signal,
        onLifecycle: (lifecycle) => {
          if (lifecycle.phase === "start") {
            this.deps.log(`[chat] trace_id=${lifecycle.traceId} phase=start`);
            return;
          }
          this.deps.log(
            `[chat] trace_id=${lifecycle.traceId} phase=terminal code=${lifecycle.terminalCode ?? "unknown"} elapsed_ms=${lifecycle.elapsedMs ?? 0}`,
          );
        },
      })) {
        if (event.kind === "chunk") {
          this.postMessage({ type: "streamChunk", id: message.id, text: event.text });
          continue;
        }
        if (event.kind === "done") {
          this.emitExecutionStep(message.id, "completed", "Execution completed.");
          this.postMessage({ type: "streamDone", id: message.id });
          continue;
        }
        const classified = classifyStreamError(event);
        this.emitExecutionStep(message.id, "failed", `Execution failed: ${classified.code}.`);
        this.postMessage({
          type: "streamError",
          id: message.id,
          message: classified.message,
          code: classified.code,
          retryable: classified.retryable,
        });
      }
    } catch (error) {
      const errText = error instanceof Error ? error.message : String(error);
      const classified = classifyStreamErrorMessage(errText);
      this.deps.log(`[chat] stream failure: ${errText}`);
      this.emitExecutionStep(message.id, "failed", `Execution failed: ${classified.code}.`);
      this.postMessage({
        type: "streamError",
        id: message.id,
        message: classified.message,
        code: classified.code,
        retryable: classified.retryable,
      });
    } finally {
      this.pendingStreams.delete(message.id);
    }
  }

  private cancelPendingStream(id: string): void {
    const pending = this.pendingStreams.get(id);
    if (pending === undefined) {
      return;
    }
    pending.controller.abort();
    this.pendingStreams.delete(id);
    this.emitExecutionStep(id, "cancelled", "Execution cancelled.");
  }

  private async handleApplyCodeBlock(
    message: Extract<WebviewToExtension, { type: "applyCodeBlock" }>,
  ): Promise<void> {
    const policy = this.modePolicy();
    if (!policy.canMutateFiles) {
      this.postMessage({ type: "applyResult", id: message.id, result: { outcome: "error", detail: "Ask mode blocks file mutations." } });
      this.postMessage({
        type: "statusMessage",
        level: "warn",
        text: "Ask mode blocks Apply actions.",
      });
      return;
    }
    if (policy.requiresMutationApproval) {
      const approved = await this.requestApproval(
        "mutation",
        "Approve file mutation",
        "Apply this code block to the active editor?",
      );
      if (!approved) {
        this.postMessage({ type: "applyResult", id: message.id, result: { outcome: "cancelled", detail: "Mutation approval was denied." } });
        return;
      }
    }
    const result = await applyEditToActiveFile(
      {
        id: message.id,
        code: message.code,
        language: message.language,
        granularity: message.granularity,
      },
      { provider: this.proposalProvider, log: this.deps.log },
    );
    this.postMessage({ type: "applyResult", id: message.id, result });
  }

  private async handleInsertCodeBlock(
    message: Extract<WebviewToExtension, { type: "insertCodeBlock" }>,
  ): Promise<void> {
    const policy = this.modePolicy();
    if (!policy.canMutateFiles) {
      this.postMessage({
        type: "statusMessage",
        level: "warn",
        text: "Ask mode blocks Insert actions.",
      });
      return;
    }
    if (policy.requiresMutationApproval) {
      const approved = await this.requestApproval(
        "mutation",
        "Approve insertion",
        "Insert this code block in the active editor?",
      );
      if (!approved) {
        this.postMessage({
          type: "statusMessage",
          level: "warn",
          text: "Insertion cancelled: approval was not granted.",
        });
        return;
      }
    }
    const editor = vscode.window.activeTextEditor;
    if (editor === undefined) {
      this.postMessage({
        type: "statusMessage",
        level: "warn",
        text: "Open a file and place the cursor to insert code.",
      });
      return;
    }
    const edits = await editor.edit((builder) => {
      if (editor.selection.isEmpty) {
        builder.insert(editor.selection.active, message.code);
      } else {
        builder.replace(editor.selection, message.code);
      }
    });
    if (!edits) {
      this.postMessage({
        type: "statusMessage",
        level: "error",
        text: "Failed to insert code into the active editor.",
      });
    }
  }

  private sendContextSnapshot(): void {
    const snapshot = snapshotActiveEditor();
    this.postMessage({ type: "contextSnapshot", context: snapshot ?? null });
  }

  private flushPendingPrefills(): void {
    while (this.pendingPrefills.length > 0) {
      const next = this.pendingPrefills.shift();
      if (next !== undefined) {
        this.postMessage({ type: "prefillPrompt", payload: next });
      }
    }
  }

  private postMessage(message: ExtensionToWebview): void {
    this.view?.webview.postMessage(message);
  }

  private modePolicy(): ModePolicy {
    return resolveModePolicy(this.mode);
  }

  private async requestApproval(scope: ApprovalScope, title: string, detail: string): Promise<boolean> {
    const id = `${scope}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    this.emitExecutionStep(id, "awaiting_approval", `${title}: ${detail}`);
    this.postMessage({
      type: "approvalRequested",
      payload: { id, scope, title, detail },
    });
    return await new Promise<boolean>((resolve) => {
      this.pendingApprovals.set(id, { resolve, scope });
    });
  }

  private resolveApproval(payload: ApprovalDecisionPayload): void {
    const pending = this.pendingApprovals.get(payload.id);
    if (pending === undefined) {
      return;
    }
    this.pendingApprovals.delete(payload.id);
    pending.resolve(payload.approved);
    this.emitExecutionStep(
      payload.id,
      payload.approved ? "running" : "blocked",
      payload.approved ? "Approval granted." : "Approval denied.",
    );
  }

  private emitExecutionStep(id: string, phase: "queued" | "running" | "awaiting_approval" | "completed" | "blocked" | "failed" | "cancelled", summary: string): void {
    this.postMessage({ type: "executionStep", payload: { id, phase, summary } });
  }
}

function renderWebviewHtml(webview: vscode.Webview, extensionUri: vscode.Uri): string {
  const nonce = createNonce();
  const scriptUri = webview.asWebviewUri(
    vscode.Uri.joinPath(extensionUri, "dist", "webview.js"),
  );
  const csp = [
    `default-src 'none'`,
    `img-src ${webview.cspSource} data:`,
    `style-src ${webview.cspSource} 'unsafe-inline'`,
    `font-src ${webview.cspSource}`,
    `script-src 'nonce-${nonce}'`,
  ].join("; ");

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta http-equiv="Content-Security-Policy" content="${csp}" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>REX Chat</title>
</head>
<body>
  <div id="rex-root"></div>
  <script nonce="${nonce}" src="${scriptUri.toString()}"></script>
</body>
</html>`;
}

function createNonce(): string {
  return crypto.randomBytes(16).toString("base64").replace(/[+/=]/g, "");
}

function mapThemeKind(theme: vscode.ColorTheme): ThemeKind {
  switch (theme.kind) {
    case vscode.ColorThemeKind.Light:
      return "light";
    case vscode.ColorThemeKind.Dark:
      return "dark";
    case vscode.ColorThemeKind.HighContrast:
      return "high-contrast";
    default:
      return "high-contrast-light";
  }
}

function buildPromptWithContext(
  prompt: string,
  context: ReturnType<typeof snapshotActiveEditor> | undefined,
): string {
  if (context === undefined) {
    return prompt;
  }
  const lines = [prompt.trim(), "", "---", `File: ${context.filePath}`, `Language: ${context.languageId}`];
  if (context.selectionText !== undefined) {
    lines.push("Selection:");
    lines.push("```");
    lines.push(context.selectionText);
    lines.push("```");
  }
  return lines.join("\n");
}

function isIncomingMessage(value: unknown): value is WebviewToExtension {
  if (value === null || typeof value !== "object") {
    return false;
  }
  const type = (value as { type?: unknown }).type;
  return typeof type === "string";
}

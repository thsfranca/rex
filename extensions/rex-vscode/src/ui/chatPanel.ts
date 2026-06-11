import * as vscode from "vscode";

import { formatAttachmentsForPrompt, pickContextAttachments } from "../editor/contextPicker";
import { snapshotActiveEditor } from "../editor/context";
import { RexProposalProvider } from "../editor/virtualDocs";
import type { CliBridgeOptions } from "../runtime/cliBridge";
import type { DaemonLifecycleState } from "../runtime/daemonLifecycle";
import { classifyStreamError, classifyStreamErrorMessage } from "../runtime/errorTaxonomy";
import { formatPlanDetailMarkdown } from "../runtime/planContent";
import { defaultPlanSavePath, validatePlanSavePath } from "../runtime/planPath";
import { resolveModePolicy } from "../runtime/modePolicy";
import type { StreamErrorCode } from "../runtime/ndjsonParser";
import { streamComplete } from "../runtime/streamClient";
import type {
  ApprovalDecisionPayload,
  ApprovalScope,
  ContextAttachment,
  ExecutionStepPayload,
  ExtensionToWebview,
  InteractionMode,
  ModePolicy,
  PromptPrefillPayload,
  ThemeKind,
  WebviewToExtension,
} from "../shared/messages";

import { postToEditorPanel } from "./editorChatPanel";
import { applyEditToActiveFile } from "./applyEdit";
import {
  reviewMultiFileProposals,
  type FileProposal,
} from "./multiFileReview";
import {
  createDefaultSession,
  deriveSessionTitle,
  SessionStore,
  type ChatSessionRecord,
  type SessionStoreSnapshot,
} from "./sessionStore";
import { buildWebviewHtml } from "./webviewHtml";

export const CHAT_VIEW_ID = "rex.chatView";
export const CHAT_VIEW_SECONDARY_ID = "rex.chatViewSecondary";

export interface ChatPanelDependencies {
  readonly context: vscode.ExtensionContext;
  readonly getCliOptions: () => CliBridgeOptions;
  readonly getModelId: () => string;
  readonly getDaemonAutoStart: () => boolean;
  readonly ensureDaemonReady: (signal?: AbortSignal) => Promise<DaemonLifecycleState>;
  readonly getDaemonState: () => DaemonLifecycleState | undefined;
  readonly log: (message: string) => void;
  readonly notifyStreamFailure?: (args: { code: StreamErrorCode; message: string }) => void;
  readonly onStreamActivity?: (hint?: string) => void;
}

type PendingStream = { readonly controller: AbortController };
type PendingApproval = {
  readonly resolve: (approved: boolean) => void;
  readonly scope: ApprovalScope;
};

export class ChatPanelProvider implements vscode.WebviewViewProvider, vscode.Disposable {
  private readonly webviews = new Set<vscode.Webview>();
  private readonly proposalProvider = new RexProposalProvider();
  private readonly disposables: vscode.Disposable[] = [];
  private readonly pendingStreams = new Map<string, PendingStream>();
  private readonly pendingApprovals = new Map<string, PendingApproval>();
  private readonly pendingPrefills: PromptPrefillPayload[] = [];
  private readonly sessionStore: SessionStore;
  private sessionSnapshot: SessionStoreSnapshot;
  private mode: InteractionMode = "ask";
  private terminalAttachments: ContextAttachment[] = [];

  constructor(private readonly deps: ChatPanelDependencies) {
    this.sessionStore = new SessionStore(deps.context);
    this.sessionSnapshot = this.sessionStore.load();
    this.mode = this.activeSession().mode;
  }

  register(): vscode.Disposable {
    const primaryRegistration = vscode.window.registerWebviewViewProvider(
      CHAT_VIEW_ID,
      this,
      { webviewOptions: { retainContextWhenHidden: true } },
    );
    const secondaryRegistration = vscode.window.registerWebviewViewProvider(
      CHAT_VIEW_SECONDARY_ID,
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
    this.disposables.push(
      primaryRegistration,
      secondaryRegistration,
      docRegistration,
      selectionListener,
      activeEditorListener,
      themeListener,
    );
    return new vscode.Disposable(() => this.dispose());
  }

  getProposalProvider(): RexProposalProvider {
    return this.proposalProvider;
  }

  async reviewMultiFileBatch(proposals: ReadonlyArray<FileProposal>): Promise<void> {
    await reviewMultiFileProposals(proposals, this.proposalProvider, this.deps.log);
  }

  async handleExternalMessage(raw: unknown): Promise<void> {
    if (!isIncomingMessage(raw)) {
      return;
    }
    await this.handleWebviewMessage(raw);
  }

  cancelActiveStream(): void {
    for (const id of this.pendingStreams.keys()) {
      this.cancelPendingStream(id);
    }
  }

  attachTerminalContext(text: string): void {
    const attachment: ContextAttachment = {
      id: `terminal-${Date.now()}`,
      kind: "terminal",
      label: "Terminal selection",
      text: text.slice(0, 16_000),
    };
    this.terminalAttachments = [...this.terminalAttachments, attachment];
    this.postMessage({ type: "contextAttachments", attachments: this.terminalAttachments });
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
    this.webviews.clear();
    this.proposalProvider.dispose();
    for (const item of this.disposables) {
      item.dispose();
    }
    this.disposables.length = 0;
  }

  resolveWebviewView(view: vscode.WebviewView): void {
    const { webview } = view;
    webview.options = {
      enableScripts: true,
      localResourceRoots: [vscode.Uri.joinPath(this.deps.context.extensionUri, "dist")],
    };
    webview.html = buildWebviewHtml(webview, this.deps.context.extensionUri);
    this.webviews.add(webview);

    const messageSub = webview.onDidReceiveMessage(async (raw: unknown) => {
      if (!isIncomingMessage(raw)) {
        return;
      }
      await this.handleWebviewMessage(raw);
    });
    const disposeSub = view.onDidDispose(() => {
      messageSub.dispose();
      disposeSub.dispose();
      this.webviews.delete(webview);
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
    if (this.webviews.size === 0) {
      this.pendingPrefills.push(payload);
      return;
    }
    this.postMessage({ type: "prefillPrompt", payload });
    void vscode.commands.executeCommand(`${CHAT_VIEW_ID}.focus`);
  }

  clearChat(): void {
    this.abortAllPendingStreams();
    this.rejectAllPendingApprovals();
    this.postMessage({ type: "clearChat" });
  }

  private activeSession(): ChatSessionRecord {
    const found = this.sessionSnapshot.sessions.find(
      (session) => session.id === this.sessionSnapshot.activeSessionId,
    );
    return found ?? createDefaultSession();
  }

  private broadcastSessions(): void {
    this.postMessage({
      type: "sessionList",
      sessions: this.sessionSnapshot.sessions.map((session) => ({
        id: session.id,
        title: session.title,
        isActive: session.id === this.sessionSnapshot.activeSessionId,
      })),
    });
    const active = this.activeSession();
    this.postMessage({
      type: "sessionMessages",
      payload: { sessionId: active.id, messages: active.messages },
    });
  }

  private async persistActiveSession(
    messages: ChatSessionRecord["messages"],
    mode: InteractionMode,
  ): Promise<void> {
    const activeId = this.sessionSnapshot.activeSessionId;
    const sessions = this.sessionSnapshot.sessions.map((session) => {
      if (session.id !== activeId) {
        return session;
      }
      const firstUser = messages.find((message) => message.role === "user");
      const title =
        session.title === "Chat" && firstUser !== undefined
          ? deriveSessionTitle(firstUser.buffer)
          : session.title;
      return {
        ...session,
        title,
        mode,
        messages,
        updatedAt: Date.now(),
      };
    });
    this.sessionSnapshot = { sessions, activeSessionId: activeId };
    await this.sessionStore.save(this.sessionSnapshot);
    this.broadcastSessions();
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
        this.broadcastSessions();
        this.postMessage({ type: "contextAttachments", attachments: this.terminalAttachments });
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
      case "createSession": {
        const session = {
          ...createDefaultSession(),
          id: `session-${Date.now()}`,
          title: "New chat",
        };
        this.sessionSnapshot = {
          sessions: [...this.sessionSnapshot.sessions, session],
          activeSessionId: session.id,
        };
        this.mode = session.mode;
        await this.sessionStore.save(this.sessionSnapshot);
        this.postMessage({ type: "clearChat" });
        this.broadcastSessions();
        this.postMessage({ type: "modeState", payload: this.modePolicy() });
        return;
      }
      case "switchSession": {
        if (this.sessionSnapshot.sessions.some((session) => session.id === message.sessionId)) {
          this.sessionSnapshot = {
            ...this.sessionSnapshot,
            activeSessionId: message.sessionId,
          };
          this.mode = this.activeSession().mode;
          await this.sessionStore.save(this.sessionSnapshot);
          this.broadcastSessions();
          this.postMessage({ type: "modeState", payload: this.modePolicy() });
        }
        return;
      }
      case "deleteSession": {
        if (this.sessionSnapshot.sessions.length <= 1) {
          return;
        }
        const sessions = this.sessionSnapshot.sessions.filter(
          (session) => session.id !== message.sessionId,
        );
        const activeSessionId =
          this.sessionSnapshot.activeSessionId === message.sessionId
            ? sessions[0].id
            : this.sessionSnapshot.activeSessionId;
        this.sessionSnapshot = { sessions, activeSessionId };
        this.mode = this.activeSession().mode;
        await this.sessionStore.save(this.sessionSnapshot);
        this.broadcastSessions();
        return;
      }
      case "saveSessionState":
        await this.persistActiveSession(message.messages, message.mode);
        return;
      case "requestContextPicker": {
        const picked = await pickContextAttachments();
        if (picked.length === 0) {
          return;
        }
        this.terminalAttachments = [...this.terminalAttachments, ...picked];
        this.postMessage({ type: "contextAttachments", attachments: this.terminalAttachments });
        return;
      }
      case "removeContextAttachment":
        this.terminalAttachments = this.terminalAttachments.filter(
          (item) => item.id !== message.id,
        );
        this.postMessage({ type: "contextAttachments", attachments: this.terminalAttachments });
        return;
      case "savePlan":
        await this.handleSavePlan(message);
        return;
      case "buildPlan":
        await this.handleBuildPlan(message);
        return;
    }
  }

  private async handleSubmitPrompt(
    message: Extract<WebviewToExtension, { type: "submitPrompt" }>,
  ): Promise<void> {
    this.cancelPendingStream(message.id);
    const controller = new AbortController();
    this.pendingStreams.set(message.id, { controller });

    const attachmentText = formatAttachmentsForPrompt(message.attachments ?? this.terminalAttachments);
    const promptForDaemon = `${message.prompt}${attachmentText}`;
    const clientHints =
      message.attachContext && message.context !== undefined
        ? {
            activeFilePath: message.context.filePath,
            languageId: message.context.languageId,
            selectionText: message.context.selectionText,
          }
        : undefined;

    this.postMessage({ type: "streamStarted", id: message.id });
    void vscode.commands.executeCommand("setContext", "rex.chatStreaming", true);
    this.emitExecutionStep(message.id, "queued", `Request queued in ${this.mode.toUpperCase()} mode.`);

    try {
      const approvalId = `apr-${message.id}`;
      if (this.modePolicy().requiresExecutionApproval) {
        const approved = await this.requestApproval(
          approvalId,
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
          this.postMessage({
            type: "streamError",
            id: message.id,
            message: detail,
            code: "daemon_unavailable",
            retryable: true,
          });
          this.deps.notifyStreamFailure?.({ code: "daemon_unavailable", message: detail });
          return;
        }
      }

      this.emitExecutionStep(message.id, "running", "Execution started.", "step");
      this.deps.onStreamActivity?.("execution");
      const configuredModel = this.deps.getModelId().trim();
      for await (const event of streamComplete(this.deps.getCliOptions(), {
        prompt: promptForDaemon,
        clientHints,
        mode: this.mode,
        model: configuredModel.length > 0 ? configuredModel : undefined,
        approvalId: this.modePolicy().requiresExecutionApproval ? approvalId : undefined,
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
        if (event.kind === "tool") {
          if (event.phase === "running") {
            this.deps.onStreamActivity?.(event.name);
          }
          this.emitExecutionStep(
            message.id,
            mapToolPhase(event.phase),
            event.name,
            "tool",
            event.detail,
            event.toolCallId,
          );
          continue;
        }
        if (event.kind === "activity") {
          this.deps.onStreamActivity?.(event.summary);
          this.emitExecutionStep(
            message.id,
            "running",
            event.summary,
            "activity",
            event.detail ?? event.phase,
          );
          continue;
        }
        if (event.kind === "step") {
          this.emitExecutionStep(message.id, "running", event.summary, "step", event.summary);
          continue;
        }
        if (event.kind === "plan") {
          const content =
            event.phase === "ready"
              ? formatPlanDetailMarkdown(event.title, event.detail)
              : event.detail;
          this.postMessage({
            type: "planArtifact",
            payload: {
              streamId: message.id,
              phase: event.phase,
              title: event.title,
              detail: event.detail,
              content,
              savePath: defaultPlanSavePath(event.title),
            },
          });
          continue;
        }
        if (event.kind === "done") {
          this.emitExecutionStep(message.id, "completed", "Execution completed.", "step");
          this.postMessage({ type: "streamDone", id: message.id });
          continue;
        }
        const classified = classifyStreamError(event);
        this.emitExecutionStep(message.id, "failed", `Execution failed: ${classified.code}.`, "step");
        this.postMessage({
          type: "streamError",
          id: message.id,
          message: classified.message,
          code: classified.code,
          retryable: classified.retryable,
        });
        this.deps.notifyStreamFailure?.({ code: classified.code, message: classified.message });
      }
    } catch (error) {
      const errText = error instanceof Error ? error.message : String(error);
      const classified = classifyStreamErrorMessage(errText);
      this.deps.log(`[chat] stream failure: ${errText}`);
      this.emitExecutionStep(message.id, "failed", `Execution failed: ${classified.code}.`, "step");
      this.postMessage({
        type: "streamError",
        id: message.id,
        message: classified.message,
        code: classified.code,
        retryable: classified.retryable,
      });
      this.deps.notifyStreamFailure?.({ code: classified.code, message: classified.message });
    } finally {
      this.pendingStreams.delete(message.id);
      if (this.pendingStreams.size === 0) {
        this.deps.onStreamActivity?.(undefined);
        void vscode.commands.executeCommand("setContext", "rex.chatStreaming", false);
      }
    }
  }

  private cancelPendingStream(id: string): void {
    const pending = this.pendingStreams.get(id);
    if (pending === undefined) {
      return;
    }
    pending.controller.abort();
    this.pendingStreams.delete(id);
    this.resolvePendingApproval(`apr-${id}`, false);
    this.emitExecutionStep(id, "cancelled", "Execution cancelled.");
  }

  private abortAllPendingStreams(): void {
    for (const pending of this.pendingStreams.values()) {
      pending.controller.abort();
    }
    this.pendingStreams.clear();
  }

  private rejectAllPendingApprovals(): void {
    for (const pending of this.pendingApprovals.values()) {
      pending.resolve(false);
    }
    this.pendingApprovals.clear();
  }

  private async handleApplyCodeBlock(
    message: Extract<WebviewToExtension, { type: "applyCodeBlock" }>,
  ): Promise<void> {
    const policy = this.modePolicy();
    if (!policy.canMutateFiles) {
      this.postMessage({
        type: "applyResult",
        id: message.id,
        result: { outcome: "error", detail: "Ask mode blocks file mutations." },
      });
      this.postMessage({
        type: "statusMessage",
        level: "warn",
        text: "Ask mode blocks Apply actions.",
      });
      return;
    }
    if (policy.requiresMutationApproval) {
      const approved = await this.requestApproval(
        `apr-mut-${message.id}`,
        "mutation",
        "Approve file mutation",
        "Apply this code block to the active editor?",
      );
      if (!approved) {
        this.postMessage({
          type: "applyResult",
          id: message.id,
          result: { outcome: "cancelled", detail: "Mutation approval was denied." },
        });
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
        `apr-ins-${Date.now()}`,
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
    for (const webview of this.webviews) {
      void webview.postMessage(message);
    }
    postToEditorPanel(message);
  }

  private modePolicy(): ModePolicy {
    return resolveModePolicy(this.mode);
  }

  private async requestApproval(
    id: string,
    scope: ApprovalScope,
    title: string,
    detail: string,
  ): Promise<boolean> {
    this.emitExecutionStep(id, "awaiting_approval", `${title}: ${detail}`, "step");
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
      "step",
    );
  }

  private resolvePendingApproval(approvalId: string, approved: boolean): void {
    const pending = this.pendingApprovals.get(approvalId);
    if (pending === undefined) {
      return;
    }
    this.pendingApprovals.delete(approvalId);
    pending.resolve(approved);
  }

  private emitExecutionStep(
    streamId: string,
    phase: ExecutionStepPayload["phase"],
    summary: string,
    kind?: ExecutionStepPayload["kind"],
    detail?: string,
    toolCallId?: string,
  ): void {
    const id = toolCallId ?? `${streamId}-${phase}-${summary}`;
    this.postMessage({
      type: "executionStep",
      payload: {
        id,
        streamId,
        toolCallId,
        phase,
        summary,
        kind,
        detail: detail ?? summary,
      },
    });
  }

  private async handleSavePlan(
    message: Extract<WebviewToExtension, { type: "savePlan" }>,
  ): Promise<void> {
    const validation = validatePlanSavePath(message.path);
    if (!validation.ok) {
      this.postMessage({
        type: "planSaveResult",
        payload: {
          streamId: message.streamId,
          ok: false,
          message: validation.message,
        },
      });
      return;
    }

    const folder = vscode.workspace.workspaceFolders?.[0];
    if (folder === undefined) {
      this.postMessage({
        type: "planSaveResult",
        payload: {
          streamId: message.streamId,
          ok: false,
          message: "Open a workspace folder to save plans under .rex/plans/.",
        },
      });
      return;
    }

    const relative = validation.normalized;
    const plansDir = vscode.Uri.joinPath(folder.uri, ".rex", "plans");
    const target = vscode.Uri.joinPath(folder.uri, ...relative.split("/"));
    try {
      await vscode.workspace.fs.createDirectory(plansDir);
      await vscode.workspace.fs.writeFile(target, Buffer.from(message.content, "utf8"));
      this.postMessage({
        type: "planSaveResult",
        payload: {
          streamId: message.streamId,
          ok: true,
          path: relative,
          message: `Plan saved to ${relative}.`,
        },
      });
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      this.postMessage({
        type: "planSaveResult",
        payload: {
          streamId: message.streamId,
          ok: false,
          message: `Failed to save plan: ${detail}`,
        },
      });
    }
  }

  private async handleBuildPlan(
    message: Extract<WebviewToExtension, { type: "buildPlan" }>,
  ): Promise<void> {
    this.mode = "agent";
    this.postMessage({ type: "modeState", payload: this.modePolicy() });
    const pathHint = message.savePath.trim().length > 0 ? message.savePath : defaultPlanSavePath(message.title);
    const prompt = [
      `Execute the approved plan "${message.title}".`,
      `Plan reference: ${pathHint}`,
      "",
      message.content.trim(),
    ].join("\n");
    this.postMessage({
      type: "prefillPrompt",
      payload: { prompt },
    });
    this.postMessage({
      type: "statusMessage",
      level: "info",
      text: "Switched to AGENT mode. Review the plan prompt and send when ready.",
    });
  }
}

function mapToolPhase(phase: string): ExecutionStepPayload["phase"] {
  if (phase === "completed") {
    return "completed";
  }
  if (phase === "failed") {
    return "failed";
  }
  return "running";
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

function isIncomingMessage(value: unknown): value is WebviewToExtension {
  if (value === null || typeof value !== "object") {
    return false;
  }
  const type = (value as { type?: unknown }).type;
  return typeof type === "string";
}

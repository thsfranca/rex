/**
 * Typed message bus contracts shared between the extension host and the webview.
 *
 * Keep this file pure (no vscode imports) so it can be bundled both in the host
 * and in the browser-targeted webview.
 */

export type StreamId = string;
export type StreamErrorCode =
  | "daemon_unavailable"
  | "sidecar_unavailable"
  | "inference_config"
  | "stream_timeout"
  | "stream_interrupted"
  | "stream_incomplete"
  | "cancelled"
  | "invalid_response"
  | "spawn_failed"
  | "unknown";

export interface PromptContextSnapshot {
  readonly filePath: string;
  readonly languageId: string;
  readonly selectionText?: string;
  readonly selectionRange?: {
    readonly startLine: number;
    readonly startCharacter: number;
    readonly endLine: number;
    readonly endCharacter: number;
  };
}

export type DaemonState = "ready" | "starting" | "unavailable";

export interface DaemonStatePayload {
  readonly state: DaemonState;
  readonly detail?: string;
}

export type ApplyGranularity = "file" | "selection";

export type ApplyOutcome = "applied" | "rejected" | "cancelled" | "error";

export interface ApplyResultPayload {
  readonly outcome: ApplyOutcome;
  readonly detail?: string;
}

export type ThemeKind = "light" | "dark" | "high-contrast" | "high-contrast-light";

export interface ThemePayload {
  readonly kind: ThemeKind;
}

export interface PromptPrefillPayload {
  readonly prompt: string;
  readonly context?: PromptContextSnapshot;
}

export type InteractionMode = "ask" | "plan" | "agent";

export type ApprovalScope = "execution" | "mutation";

export interface ModePolicy {
  readonly mode: InteractionMode;
  readonly canMutateFiles: boolean;
  readonly requiresExecutionApproval: boolean;
  readonly requiresMutationApproval: boolean;
  readonly summary: string;
}

export interface ApprovalRequestPayload {
  readonly id: string;
  readonly scope: ApprovalScope;
  readonly title: string;
  readonly detail: string;
  /** Optional before/after snippets for in-webview diff preview. */
  readonly edits?: ReadonlyArray<FileEditPreview>;
}

export interface FileEditPreview {
  readonly filePath: string;
  readonly languageId?: string;
  readonly before: string;
  readonly after: string;
}

export interface ApprovalDecisionPayload {
  readonly id: string;
  readonly approved: boolean;
}

export interface ExecutionStepPayload {
  readonly id: string;
  readonly streamId?: string;
  readonly toolCallId?: string;
  readonly phase: "queued" | "running" | "awaiting_approval" | "completed" | "blocked" | "failed" | "cancelled";
  readonly summary: string;
  readonly kind?: "tool" | "step" | "activity";
  readonly detail?: string;
}

export type PlanStreamPhase = "draft" | "clarify" | "ready";

export interface PlanArtifactPayload {
  readonly streamId: StreamId;
  readonly phase: PlanStreamPhase;
  readonly title: string;
  readonly detail: string;
  readonly content: string;
  readonly savePath: string;
}

export interface PlanSaveResultPayload {
  readonly streamId: StreamId;
  readonly ok: boolean;
  readonly path?: string;
  readonly message: string;
}

export interface ContextAttachment {
  readonly id: string;
  readonly kind: "file" | "symbol" | "terminal";
  readonly label: string;
  readonly text: string;
}

export interface SessionSummary {
  readonly id: string;
  readonly title: string;
  readonly isActive: boolean;
}

export interface SessionMessagesPayload {
  readonly sessionId: string;
  readonly messages: ReadonlyArray<{
    readonly id: string;
    readonly role: "user" | "assistant";
    readonly buffer: string;
    readonly errorMessage?: string;
  }>;
}

export type ExtensionToWebview =
  | { readonly type: "streamStarted"; readonly id: StreamId }
  | { readonly type: "streamChunk"; readonly id: StreamId; readonly text: string }
  | { readonly type: "streamDone"; readonly id: StreamId }
  | {
      readonly type: "streamError";
      readonly id: StreamId;
      readonly message: string;
      readonly code?: StreamErrorCode;
      readonly retryable?: boolean;
    }
  | { readonly type: "daemonState"; readonly payload: DaemonStatePayload }
  | { readonly type: "theme"; readonly payload: ThemePayload }
  | { readonly type: "contextSnapshot"; readonly context: PromptContextSnapshot | null }
  | { readonly type: "prefillPrompt"; readonly payload: PromptPrefillPayload }
  | { readonly type: "applyResult"; readonly id: StreamId; readonly result: ApplyResultPayload }
  | { readonly type: "modeState"; readonly payload: ModePolicy }
  | { readonly type: "approvalRequested"; readonly payload: ApprovalRequestPayload }
  | { readonly type: "executionStep"; readonly payload: ExecutionStepPayload }
  | { readonly type: "planArtifact"; readonly payload: PlanArtifactPayload }
  | { readonly type: "planSaveResult"; readonly payload: PlanSaveResultPayload }
  | { readonly type: "clearChat" }
  | { readonly type: "statusMessage"; readonly level: "info" | "warn" | "error"; readonly text: string }
  | { readonly type: "sessionList"; readonly sessions: ReadonlyArray<SessionSummary> }
  | { readonly type: "sessionMessages"; readonly payload: SessionMessagesPayload }
  | { readonly type: "contextAttachments"; readonly attachments: ReadonlyArray<ContextAttachment> };

export type WebviewToExtension =
  | { readonly type: "ready" }
  | {
      readonly type: "submitPrompt";
      readonly id: StreamId;
      readonly prompt: string;
      readonly context?: PromptContextSnapshot;
      readonly attachContext: boolean;
      readonly attachments?: ReadonlyArray<ContextAttachment>;
    }
  | { readonly type: "cancelStream"; readonly id: StreamId }
  | {
      readonly type: "applyCodeBlock";
      readonly id: StreamId;
      readonly language: string;
      readonly code: string;
      readonly granularity: ApplyGranularity;
    }
  | { readonly type: "insertCodeBlock"; readonly code: string }
  | { readonly type: "copyCodeBlock"; readonly code: string }
  | { readonly type: "setMode"; readonly mode: InteractionMode }
  | { readonly type: "approvalDecision"; readonly payload: ApprovalDecisionPayload }
  | { readonly type: "requestContextSnapshot" }
  | { readonly type: "clearChatRequested" }
  | { readonly type: "createSession" }
  | { readonly type: "switchSession"; readonly sessionId: string }
  | { readonly type: "deleteSession"; readonly sessionId: string }
  | { readonly type: "saveSessionState"; readonly sessionId: string; readonly messages: SessionMessagesPayload["messages"]; readonly mode: InteractionMode }
  | { readonly type: "requestContextPicker" }
  | { readonly type: "removeContextAttachment"; readonly id: string }
  | { readonly type: "savePlan"; readonly streamId: StreamId; readonly path: string; readonly content: string }
  | { readonly type: "buildPlan"; readonly streamId: StreamId; readonly title: string; readonly content: string; readonly savePath: string };

/**
 * Typed message bus contracts shared between the extension host and the webview.
 *
 * Keep this file pure (no vscode imports) so it can be bundled both in the host
 * and in the browser-targeted webview.
 */

export type StreamId = string;

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

export type ExtensionToWebview =
  | { readonly type: "streamStarted"; readonly id: StreamId }
  | { readonly type: "streamChunk"; readonly id: StreamId; readonly text: string }
  | { readonly type: "streamDone"; readonly id: StreamId }
  | { readonly type: "streamError"; readonly id: StreamId; readonly message: string }
  | { readonly type: "daemonState"; readonly payload: DaemonStatePayload }
  | { readonly type: "theme"; readonly payload: ThemePayload }
  | { readonly type: "contextSnapshot"; readonly context: PromptContextSnapshot | null }
  | { readonly type: "prefillPrompt"; readonly payload: PromptPrefillPayload }
  | { readonly type: "applyResult"; readonly id: StreamId; readonly result: ApplyResultPayload }
  | { readonly type: "clearChat" }
  | { readonly type: "statusMessage"; readonly level: "info" | "warn" | "error"; readonly text: string };

export type WebviewToExtension =
  | { readonly type: "ready" }
  | {
      readonly type: "submitPrompt";
      readonly id: StreamId;
      readonly prompt: string;
      readonly context?: PromptContextSnapshot;
      readonly attachContext: boolean;
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
  | { readonly type: "requestContextSnapshot" }
  | { readonly type: "clearChatRequested" };

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

export type ExtensionToWebview =
  | { readonly type: "streamStarted"; readonly id: StreamId }
  | { readonly type: "streamChunk"; readonly id: StreamId; readonly text: string }
  | { readonly type: "streamDone"; readonly id: StreamId }
  | { readonly type: "streamError"; readonly id: StreamId; readonly message: string }
  | { readonly type: "daemonState"; readonly payload: DaemonStatePayload };

export type WebviewToExtension =
  | {
      readonly type: "submitPrompt";
      readonly id: StreamId;
      readonly prompt: string;
      readonly context?: PromptContextSnapshot;
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
  | { readonly type: "copyCodeBlock"; readonly code: string };

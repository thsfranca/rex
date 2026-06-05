import type {
  ApprovalRequestPayload,
  ApplyResultPayload,
  ContextAttachment,
  DaemonStatePayload,
  ExtensionToWebview,
  InteractionMode,
  ModePolicy,
  PlanArtifactPayload,
  PromptContextSnapshot,
  SessionSummary,
  ThemeKind,
} from "../src/shared/messages";

import type { RenderedMessage } from "./renderedMessage";
import { MarkdownStream } from "./streaming/markdownStream";

export interface BannerState {
  readonly level: "info" | "warn" | "error";
  readonly text: string;
}

export interface AppState {
  messages: RenderedMessage[];
  streams: Map<string, MarkdownStream>;
  applyResults: Map<string, Map<string, ApplyResultPayload>>;
  daemon: DaemonStatePayload;
  theme: ThemeKind;
  context: PromptContextSnapshot | null;
  attachContext: boolean;
  prompt: string;
  streaming: boolean;
  activeStreamId?: string;
  banner?: BannerState;
  modePolicy: ModePolicy;
  pendingApprovals: ApprovalRequestPayload[];
  timeline: { id: string; summary: string; phase: string; kind?: string; detail?: string }[];
  planArtifact?: PlanArtifactPayload;
  sessions: SessionSummary[];
  attachments: ContextAttachment[];
  activeSessionId?: string;
}

export type Action =
  | { type: "hostMessage"; payload: ExtensionToWebview }
  | { type: "setPrompt"; value: string }
  | { type: "setAttachContext"; value: boolean }
  | { type: "setMode"; value: InteractionMode }
  | { type: "userSend"; id: string; text: string; context?: PromptContextSnapshot; attachContext: boolean }
  | { type: "clearChat" }
  | { type: "cancelStream" }
  | { type: "approvalDecision"; id: string; approved: boolean }
  | { type: "clearBanner" }
  | { type: "updatePlanContent"; content: string }
  | { type: "updatePlanSavePath"; path: string };

export const initialState: AppState = {
  messages: [],
  streams: new Map(),
  applyResults: new Map(),
  daemon: { state: "unavailable", detail: "probing" },
  theme: "dark",
  context: null,
  attachContext: false,
  prompt: "",
  streaming: false,
  modePolicy: {
    mode: "ask",
    canMutateFiles: false,
    requiresExecutionApproval: false,
    requiresMutationApproval: true,
    summary: "Research and explain only. File mutations are blocked.",
  },
  pendingApprovals: [],
  timeline: [],
  sessions: [],
  attachments: [],
};

export function reducer(state: AppState, action: Action): AppState {
  switch (action.type) {
    case "setPrompt":
      return { ...state, prompt: action.value };
    case "setAttachContext":
      return { ...state, attachContext: action.value };
    case "setMode":
      return { ...state, modePolicy: { ...state.modePolicy, mode: action.value } };
    case "clearBanner":
      return { ...state, banner: undefined };
    case "userSend": {
      const userMessage: RenderedMessage = {
        id: `user-${action.id}`,
        role: "user",
        buffer: action.text,
        trailingRaw: "",
        streaming: false,
      };
      const assistantMessage: RenderedMessage = {
        id: action.id,
        role: "assistant",
        buffer: "",
        trailingRaw: "",
        streaming: true,
      };
      const streams = new Map(state.streams);
      streams.set(action.id, new MarkdownStream());
      return {
        ...state,
        messages: [...state.messages, userMessage, assistantMessage],
        streams,
        streaming: true,
        activeStreamId: action.id,
        prompt: "",
        planArtifact: undefined,
      };
    }
    case "clearChat":
      return {
        ...state,
        messages: [],
        streams: new Map(),
        applyResults: new Map(),
        streaming: false,
        activeStreamId: undefined,
        planArtifact: undefined,
      };
    case "cancelStream":
      return state;
    case "approvalDecision":
      return {
        ...state,
        pendingApprovals: state.pendingApprovals.filter((pending) => pending.id !== action.id),
      };
    case "updatePlanContent":
      if (state.planArtifact === undefined) {
        return state;
      }
      return {
        ...state,
        planArtifact: { ...state.planArtifact, content: action.content },
      };
    case "updatePlanSavePath":
      if (state.planArtifact === undefined) {
        return state;
      }
      return {
        ...state,
        planArtifact: { ...state.planArtifact, savePath: action.path },
      };
    case "hostMessage":
      return handleHostMessage(state, action.payload);
  }
}

function handleHostMessage(state: AppState, message: ExtensionToWebview): AppState {
  switch (message.type) {
    case "daemonState":
      return { ...state, daemon: message.payload };
    case "theme":
      return { ...state, theme: message.payload.kind };
    case "modeState":
      return { ...state, modePolicy: message.payload };
    case "contextSnapshot":
      return { ...state, context: message.context };
    case "streamStarted":
      return { ...state, planArtifact: undefined };
    case "streamChunk": {
      const stream = state.streams.get(message.id) ?? new MarkdownStream();
      stream.push(message.text);
      const updated = updateAssistantMessage(state.messages, message.id, (msg) => ({
        ...msg,
        buffer: msg.buffer + message.text,
        trailingRaw: computeTrailing(stream.push("").trailingRaw, msg.buffer + message.text),
        streaming: true,
      }));
      const streams = new Map(state.streams);
      streams.set(message.id, stream);
      return { ...state, messages: updated, streams };
    }
    case "streamDone": {
      const stream = state.streams.get(message.id);
      stream?.finalize();
      const updated = updateAssistantMessage(state.messages, message.id, (msg) => ({
        ...msg,
        trailingRaw: "",
        streaming: false,
      }));
      const clearsActive = message.id === state.activeStreamId;
      return {
        ...state,
        messages: updated,
        streaming: clearsActive ? false : state.streaming,
        activeStreamId: clearsActive ? undefined : state.activeStreamId,
      };
    }
    case "streamError": {
      const updated = updateAssistantMessage(state.messages, message.id, (msg) => ({
        ...msg,
        streaming: false,
        errorMessage: message.message,
      }));
      const clearsActive = message.id === state.activeStreamId;
      return {
        ...state,
        messages: updated,
        streaming: clearsActive ? false : state.streaming,
        activeStreamId: clearsActive ? undefined : state.activeStreamId,
      };
    }
    case "applyResult": {
      const messageId = extractMessageId(message.id);
      const existing = state.applyResults.get(messageId) ?? new Map();
      const nextMap = new Map(existing);
      nextMap.set(message.id, message.result);
      const nextResults = new Map(state.applyResults);
      nextResults.set(messageId, nextMap);
      const updated = state.messages.map((msg) =>
        msg.id === messageId ? { ...msg, applyResults: nextMap } : msg,
      );
      return { ...state, messages: updated, applyResults: nextResults };
    }
    case "prefillPrompt":
      return {
        ...state,
        prompt: message.payload.prompt,
        context: message.payload.context ?? state.context,
      };
    case "clearChat":
      return {
        ...state,
        messages: [],
        streams: new Map(),
        applyResults: new Map(),
        streaming: false,
        activeStreamId: undefined,
        planArtifact: undefined,
      };
    case "statusMessage":
      return {
        ...state,
        banner: { level: message.level, text: message.text },
      };
    case "approvalRequested":
      return {
        ...state,
        pendingApprovals: [...state.pendingApprovals, message.payload],
      };
    case "executionStep":
      return {
        ...state,
        timeline: [
          ...state.timeline,
          {
            id: message.payload.id,
            phase: message.payload.phase,
            summary: message.payload.summary,
            kind: message.payload.kind,
            detail: message.payload.detail,
          },
        ].slice(-20),
      };
    case "planArtifact":
      return {
        ...state,
        planArtifact: message.payload,
      };
    case "planSaveResult":
      return {
        ...state,
        banner: {
          level: message.payload.ok ? "info" : "error",
          text: message.payload.message,
        },
        planArtifact:
          message.payload.ok && state.planArtifact !== undefined && message.payload.path !== undefined
            ? { ...state.planArtifact, savePath: message.payload.path }
            : state.planArtifact,
      };
    case "sessionList":
      return {
        ...state,
        sessions: [...message.sessions],
        activeSessionId: message.sessions.find((session) => session.isActive)?.id,
      };
    case "sessionMessages": {
      const restored = message.payload.messages.map((entry) => ({
        id: entry.id,
        role: entry.role,
        buffer: entry.buffer,
        trailingRaw: "",
        streaming: false,
        errorMessage: entry.errorMessage,
      }));
      return {
        ...state,
        messages: restored,
        streams: new Map(),
        applyResults: new Map(),
        streaming: false,
        activeStreamId: undefined,
        activeSessionId: message.payload.sessionId,
      };
    }
    case "contextAttachments":
      return { ...state, attachments: [...message.attachments] };
  }
}

function computeTrailing(trailingFromStream: string, fullBuffer: string): string {
  if (trailingFromStream.length === 0) {
    return "";
  }
  return fullBuffer.slice(fullBuffer.length - trailingFromStream.length);
}

function updateAssistantMessage(
  messages: ReadonlyArray<RenderedMessage>,
  id: string,
  mutator: (msg: RenderedMessage) => RenderedMessage,
): RenderedMessage[] {
  return messages.map((msg) => (msg.id === id ? mutator(msg) : msg));
}

function extractMessageId(codeBlockId: string): string {
  const markerIndex = codeBlockId.indexOf("-code-");
  if (markerIndex === -1) {
    return codeBlockId;
  }
  return codeBlockId.slice(0, markerIndex);
}

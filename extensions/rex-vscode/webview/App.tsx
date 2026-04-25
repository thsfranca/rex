import * as React from "react";

import type {
  ApprovalRequestPayload,
  ApplyGranularity,
  ApplyResultPayload,
  DaemonStatePayload,
  ExtensionToWebview,
  InteractionMode,
  ModePolicy,
  PromptContextSnapshot,
  ThemeKind,
} from "../src/shared/messages";

import { Chat } from "./components/Chat";
import type { RenderedMessage } from "./components/Message";
import { postToHost, subscribeToHost } from "./messageBus";
import { MarkdownStream } from "./streaming/markdownStream";

interface BannerState {
  readonly level: "info" | "warn" | "error";
  readonly text: string;
}

interface AppState {
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
  timeline: { id: string; summary: string; phase: string }[];
}

type Action =
  | { type: "hostMessage"; payload: ExtensionToWebview }
  | { type: "setPrompt"; value: string }
  | { type: "setAttachContext"; value: boolean }
  | { type: "setMode"; value: InteractionMode }
  | { type: "userSend"; id: string; text: string; context?: PromptContextSnapshot; attachContext: boolean }
  | { type: "clearChat" }
  | { type: "cancelStream" }
  | { type: "approvalDecision"; id: string; approved: boolean }
  | { type: "clearBanner" };

const initialState: AppState = {
  messages: [],
  streams: new Map(),
  applyResults: new Map(),
  daemon: { state: "unavailable", detail: "probing" },
  theme: "dark",
  context: null,
  attachContext: true,
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
};

function reducer(state: AppState, action: Action): AppState {
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
      };
    case "cancelStream":
      return state;
    case "approvalDecision":
      return {
        ...state,
        pendingApprovals: state.pendingApprovals.filter((pending) => pending.id !== action.id),
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
      return state;
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
      return { ...state, messages: updated, streaming: false, activeStreamId: undefined };
    }
    case "streamError": {
      const updated = updateAssistantMessage(state.messages, message.id, (msg) => ({
        ...msg,
        streaming: false,
        errorMessage: message.message,
      }));
      return { ...state, messages: updated, streaming: false, activeStreamId: undefined };
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
          { id: message.payload.id, phase: message.payload.phase, summary: message.payload.summary },
        ].slice(-20),
      };
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

export function App(): React.ReactElement {
  const [state, dispatch] = React.useReducer(reducer, initialState);

  React.useEffect(() => {
    const unsubscribe = subscribeToHost((message) => dispatch({ type: "hostMessage", payload: message }));
    postToHost({ type: "ready" });
    return unsubscribe;
  }, []);

  React.useEffect(() => {
    if (state.banner === undefined) {
      return;
    }
    const timer = window.setTimeout(() => dispatch({ type: "clearBanner" }), 3500);
    return () => window.clearTimeout(timer);
  }, [state.banner]);

  const handleSubmit = (): void => {
    const trimmed = state.prompt.trim();
    if (trimmed.length === 0) {
      return;
    }
    const id = `stream-${Date.now()}-${Math.random().toString(16).slice(2, 8)}`;
    dispatch({
      type: "userSend",
      id,
      text: trimmed,
      context: state.context ?? undefined,
      attachContext: state.attachContext,
    });
    postToHost({
      type: "submitPrompt",
      id,
      prompt: trimmed,
      context: state.context ?? undefined,
      attachContext: state.attachContext,
    });
  };

  const handleCancel = (): void => {
    if (state.activeStreamId === undefined) {
      return;
    }
    postToHost({ type: "cancelStream", id: state.activeStreamId });
    dispatch({ type: "cancelStream" });
  };

  const handleClear = (): void => {
    postToHost({ type: "clearChatRequested" });
    dispatch({ type: "clearChat" });
  };

  const handleModeChange = (mode: InteractionMode): void => {
    dispatch({ type: "setMode", value: mode });
    postToHost({ type: "setMode", mode });
  };

  const handleApprovalDecision = (id: string, approved: boolean): void => {
    dispatch({ type: "approvalDecision", id, approved });
    postToHost({ type: "approvalDecision", payload: { id, approved } });
  };

  const handleCopy = (code: string): void => {
    postToHost({ type: "copyCodeBlock", code });
  };

  const handleInsert = (code: string): void => {
    postToHost({ type: "insertCodeBlock", code });
  };

  const handleApply = (args: {
    id: string;
    language: string;
    code: string;
    granularity: ApplyGranularity;
  }): void => {
    postToHost({
      type: "applyCodeBlock",
      id: args.id,
      language: args.language,
      code: args.code,
      granularity: args.granularity,
    });
  };

  return (
    <>
      {state.banner !== undefined ? (
        <div
          className={`rex-status-banner rex-status-banner--${state.banner.level}`}
          role={state.banner.level === "error" ? "alert" : "status"}
        >
          {state.banner.text}
        </div>
      ) : null}
      <Chat
        messages={state.messages}
        theme={state.theme}
        context={state.context}
        attachContext={state.attachContext}
        streaming={state.streaming}
        daemonReady={state.daemon.state === "ready"}
        modePolicy={state.modePolicy}
        timeline={state.timeline}
        pendingApprovals={state.pendingApprovals}
        prompt={state.prompt}
        onPromptChange={(value) => dispatch({ type: "setPrompt", value })}
        onAttachContextChange={(value) => dispatch({ type: "setAttachContext", value })}
        onSubmit={handleSubmit}
        onCancel={handleCancel}
        onClear={handleClear}
        onModeChange={handleModeChange}
        onApprovalDecision={handleApprovalDecision}
        onCopy={handleCopy}
        onInsert={handleInsert}
        onApply={handleApply}
      />
    </>
  );
}

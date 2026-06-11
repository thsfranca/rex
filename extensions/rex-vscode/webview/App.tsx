import * as React from "react";

import type {
  ApplyGranularity,
  InteractionMode,
} from "../src/shared/messages";

import { initialState, reducer } from "./appState";
import { Chat } from "./components/Chat";
import { postToHost, subscribeToHost } from "./messageBus";

function parseSlashCommand(raw: string): { command: string | null; prompt: string } {
  const trimmed = raw.trim();
  if (!trimmed.startsWith("/")) {
    return { command: null, prompt: raw };
  }
  const [first, ...rest] = trimmed.split(/\s+/);
  return { command: first.toLowerCase(), prompt: rest.join(" ").trim() };
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

  React.useEffect(() => {
    if (state.activeSessionId === undefined) {
      return;
    }
    const persistable = state.messages
      .filter((message) => message.role === "user" || message.role === "assistant")
      .map((message) => ({
        id: message.id,
        role: message.role as "user" | "assistant",
        buffer: message.buffer,
        errorMessage: message.errorMessage,
      }));
    postToHost({
      type: "saveSessionState",
      sessionId: state.activeSessionId,
      mode: state.modePolicy.mode,
      messages: persistable,
    });
  }, [state.messages, state.modePolicy.mode, state.activeSessionId]);

  const handleSubmit = (): void => {
    const { command, prompt } = parseSlashCommand(state.prompt);
    if (command === "/clear") {
      handleClear();
      return;
    }
    if (command === "/ask" || command === "/plan" || command === "/agent") {
      const mode = command.slice(1) as InteractionMode;
      dispatch({ type: "setMode", value: mode });
      postToHost({ type: "setMode", mode });
      if (prompt.length === 0) {
        dispatch({ type: "setPrompt", value: "" });
        return;
      }
    }
    const trimmed = (command === null ? state.prompt : prompt).trim();
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
      attachments: state.attachments,
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
        attachments={state.attachments}
        sessions={state.sessions}
        streaming={state.streaming}
        daemonReady={state.daemon.state === "ready"}
        modePolicy={state.modePolicy}
        timeline={state.timeline.filter(
          (entry) =>
            state.activeStreamId === undefined ||
            entry.streamId === undefined ||
            entry.streamId === state.activeStreamId,
        )}
        activityHint={state.activityHint}
        planArtifact={state.planArtifact}
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
        onCreateSession={() => postToHost({ type: "createSession" })}
        onSwitchSession={(sessionId) => postToHost({ type: "switchSession", sessionId })}
        onRequestContextPicker={() => postToHost({ type: "requestContextPicker" })}
        onRemoveAttachment={(id) => postToHost({ type: "removeContextAttachment", id })}
        onPlanContentChange={(content) => dispatch({ type: "updatePlanContent", content })}
        onPlanSavePathChange={(path) => dispatch({ type: "updatePlanSavePath", path })}
        onPlanSave={() => {
          if (state.planArtifact === undefined) {
            return;
          }
          postToHost({
            type: "savePlan",
            streamId: state.planArtifact.streamId,
            path: state.planArtifact.savePath,
            content: state.planArtifact.content,
          });
        }}
        onPlanBuild={() => {
          if (state.planArtifact === undefined) {
            return;
          }
          postToHost({
            type: "buildPlan",
            streamId: state.planArtifact.streamId,
            title: state.planArtifact.title,
            content: state.planArtifact.content,
            savePath: state.planArtifact.savePath,
          });
        }}
      />
    </>
  );
}

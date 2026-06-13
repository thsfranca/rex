import * as React from "react";

import type {
  ApplyGranularity,
  ContextAttachment,
  InteractionMode,
  ModePolicy,
  PromptContextSnapshot,
  SessionSummary,
  ThemeKind,
} from "../../src/shared/messages";

import type { ApprovalRequestPayload, PlanArtifactPayload } from "../../src/shared/messages";

import { ApprovalSection } from "./ApprovalSection";
import { SendIcon, StopIcon } from "./icons";
import { Message, type RenderedMessage } from "./Message";
import { PlanCard } from "./PlanCard";
import { ToolCard } from "./ToolCard";
import { isTimelineNoise } from "../timeline/executionLabel";

const MODES: ReadonlyArray<{ value: InteractionMode; label: string }> = [
  { value: "ask", label: "Ask" },
  { value: "plan", label: "Plan" },
  { value: "agent", label: "Agent" },
];

type MainView = "chat" | "plan";

export interface ChatProps {
  readonly messages: ReadonlyArray<RenderedMessage>;
  readonly theme: ThemeKind;
  readonly context: PromptContextSnapshot | null;
  readonly attachContext: boolean;
  readonly attachments: ReadonlyArray<ContextAttachment>;
  readonly sessions: ReadonlyArray<SessionSummary>;
  readonly streaming: boolean;
  readonly daemonReady: boolean;
  readonly modePolicy: ModePolicy;
  readonly timeline: ReadonlyArray<{
    id: string;
    toolCallId?: string;
    summary: string;
    phase: string;
    kind?: string;
    detail?: string;
    target?: string;
  }>;
  readonly activityHint?: string;
  readonly planArtifact?: PlanArtifactPayload;
  readonly pendingApprovals: ReadonlyArray<ApprovalRequestPayload>;
  readonly prompt: string;
  readonly onPromptChange: (value: string) => void;
  readonly onAttachContextChange: (value: boolean) => void;
  readonly onSubmit: () => void;
  readonly onCancel: () => void;
  readonly onClear: () => void;
  readonly onModeChange: (mode: InteractionMode) => void;
  readonly onApprovalDecision: (id: string, approved: boolean) => void;
  readonly onCopy: (code: string) => void;
  readonly onInsert: (code: string) => void;
  readonly onApply: (args: {
    id: string;
    language: string;
    code: string;
    granularity: ApplyGranularity;
  }) => void;
  readonly onCreateSession: () => void;
  readonly onSwitchSession: (sessionId: string) => void;
  readonly onRequestContextPicker: () => void;
  readonly onRemoveAttachment: (id: string) => void;
  readonly onPlanContentChange: (content: string) => void;
  readonly onPlanSavePathChange: (path: string) => void;
  readonly onPlanSave: () => void;
  readonly onPlanBuild: () => void;
}

function composerPlaceholder(mode: InteractionMode): string {
  switch (mode) {
    case "plan":
      return "Describe what you want to plan…";
    case "agent":
      return "Ask the agent to help with your code…";
    default:
      return "Ask a question…";
  }
}

function composerStatusHint(args: {
  streaming: boolean;
  activityHint?: string;
  canSend: boolean;
  daemonReady: boolean;
}): string {
  if (args.streaming) {
    return args.activityHint !== undefined ? `${args.activityHint}…` : "Generating…";
  }
  if (!args.daemonReady) {
    return "Daemon unavailable";
  }
  if (args.canSend) {
    return "Enter to send";
  }
  return "Type a prompt";
}

function planTabLabel(artifact: PlanArtifactPayload): string {
  const title = artifact.title.trim();
  if (title.length === 0) {
    return "Plan";
  }
  return title.length > 28 ? `${title.slice(0, 28)}…` : title;
}

export function Chat(props: ChatProps): React.ReactElement {
  const listRef = React.useRef<HTMLDivElement | null>(null);
  const [mainView, setMainView] = React.useState<MainView>("chat");
  const seenPlanStreamId = React.useRef<string | undefined>(undefined);

  React.useEffect(() => {
    const el = listRef.current;
    if (el === null || mainView !== "chat") {
      return;
    }
    el.scrollTop = el.scrollHeight;
  }, [props.messages, mainView]);

  React.useEffect(() => {
    if (props.planArtifact === undefined) {
      seenPlanStreamId.current = undefined;
      setMainView("chat");
      return;
    }
    if (props.planArtifact.streamId !== seenPlanStreamId.current) {
      seenPlanStreamId.current = props.planArtifact.streamId;
      setMainView("plan");
    }
  }, [props.planArtifact]);

  const canSend = props.prompt.trim().length > 0 && !props.streaming && props.daemonReady;
  const canMutateFiles = props.modePolicy.canMutateFiles;
  const hasPlan = props.planArtifact !== undefined;

  const handleKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>): void => {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      if (canSend) {
        props.onSubmit();
      }
      return;
    }
    if (event.key === "@" && props.prompt.length === 0) {
      event.preventDefault();
      props.onRequestContextPicker();
    }
  };

  return (
    <div className="rex-app" role="region" aria-label="REX Chat">
      <header className="rex-header">
        <span className="rex-header__brand" aria-live="polite">
          <span
            className={`rex-status-dot ${
              props.daemonReady ? "rex-status-dot--ready" : "rex-status-dot--unavailable"
            }`}
            title={props.daemonReady ? "Daemon ready" : "Daemon unavailable"}
          />
          <span className="rex-header__title">REX</span>
          {!props.daemonReady ? (
            <span className="rex-header__status-text">Unavailable</span>
          ) : null}
        </span>
        <span className="rex-header__actions">
          <button type="button" className="rex-header__action" onClick={props.onCreateSession} aria-label="New chat session">
            New
          </button>
          <button type="button" className="rex-header__action" onClick={props.onClear} aria-label="Clear chat">
            Clear
          </button>
        </span>
      </header>
      {props.sessions.length > 1 ? (
        <div className="rex-session-bar" role="tablist" aria-label="Chat sessions">
          {props.sessions.map((session) => (
            <button
              key={session.id}
              type="button"
              role="tab"
              aria-selected={session.isActive}
              className={session.isActive ? "rex-session-bar__tab rex-session-bar__tab--active" : "rex-session-bar__tab"}
              onClick={() => props.onSwitchSession(session.id)}
            >
              {session.title}
            </button>
          ))}
        </div>
      ) : null}
      {hasPlan ? (
        <div className="rex-view-tabs" role="tablist" aria-label="Chat views">
          <button
            type="button"
            role="tab"
            id="rex-view-tab-chat"
            aria-selected={mainView === "chat"}
            aria-controls="rex-view-panel-chat"
            className={mainView === "chat" ? "rex-view-tab rex-view-tab--active" : "rex-view-tab"}
            onClick={() => setMainView("chat")}
          >
            Chat
          </button>
          <button
            type="button"
            role="tab"
            id="rex-view-tab-plan"
            aria-selected={mainView === "plan"}
            aria-controls="rex-view-panel-plan"
            className={mainView === "plan" ? "rex-view-tab rex-view-tab--active" : "rex-view-tab"}
            onClick={() => setMainView("plan")}
          >
            {planTabLabel(props.planArtifact!)}
          </button>
        </div>
      ) : null}
      <div className="rex-main">
        <ApprovalSection approvals={props.pendingApprovals} onDecision={props.onApprovalDecision} />
        {mainView === "chat" || !hasPlan ? (
          <>
            <div
              ref={listRef}
              id="rex-view-panel-chat"
              role="tabpanel"
              aria-labelledby={hasPlan ? "rex-view-tab-chat" : undefined}
              className="rex-messages"
              aria-live="polite"
            >
              {props.messages.length === 0 ? (
                <div className="rex-empty">
                  <p className="rex-empty__title">How can I help?</p>
                  <p className="rex-hint">
                    Ask about your code, use @ to attach context, or run editor commands to prefill a prompt.
                  </p>
                </div>
              ) : (
                props.messages.map((message) => (
                  <Message
                    key={message.id}
                    message={message}
                    theme={props.theme}
                    canMutateFiles={canMutateFiles}
                    onCopy={props.onCopy}
                    onInsert={props.onInsert}
                    onApply={props.onApply}
                  />
                ))
              )}
            </div>
            <div className="rex-composer-area">
              {props.context !== null ? (
                <div className="rex-context-chip" role="group" aria-label="Editor context">
                  <label className="rex-context-chip__attach">
                    <input
                      type="checkbox"
                      checked={props.attachContext}
                      onChange={(event) => props.onAttachContextChange(event.target.checked)}
                      aria-label="Attach editor context"
                    />
                    Attach
                  </label>
                  <div className="rex-context-chip__meta">
                    <span className="rex-context-chip__path" title={props.context.filePath}>
                      {props.context.filePath}
                    </span>
                    {props.context.selectionText !== undefined ? (
                      <span className="rex-context-chip__selection">
                        Selection: {props.context.selectionText.slice(0, 60)}
                        {props.context.selectionText.length > 60 ? "…" : ""}
                      </span>
                    ) : (
                      <span className="rex-context-chip__selection">No selection</span>
                    )}
                  </div>
                </div>
              ) : null}
              {props.attachments.length > 0 ? (
                <div className="rex-attachment-list" aria-label="Attached context">
                  {props.attachments.map((attachment) => (
                    <span key={attachment.id} className="rex-attachment-chip">
                      @{attachment.label}
                      <button
                        type="button"
                        aria-label={`Remove ${attachment.label}`}
                        onClick={() => props.onRemoveAttachment(attachment.id)}
                      >
                        ×
                      </button>
                    </span>
                  ))}
                </div>
              ) : null}
              {props.timeline.some((entry) => !isTimelineNoise(entry)) ? (
                <div className="rex-timeline" role="log" aria-live="polite" aria-label="Tool activity">
                  {props.timeline
                    .filter((entry) => !isTimelineNoise(entry))
                    .map((entry) => (
                      <ToolCard
                        key={entry.toolCallId ?? entry.id}
                        id={entry.id}
                        phase={entry.phase}
                        summary={entry.summary}
                        kind={entry.kind}
                        detail={entry.detail}
                        target={entry.target}
                      />
                    ))}
                </div>
              ) : null}
              <div className="rex-composer-shell">
                <textarea
                  className="rex-composer__input"
                  value={props.prompt}
                  onChange={(event) => props.onPromptChange(event.target.value)}
                  onKeyDown={handleKeyDown}
                  placeholder={composerPlaceholder(props.modePolicy.mode)}
                  aria-label="Prompt"
                  rows={3}
                />
                <div className="rex-composer__toolbar">
                  <div className="rex-mode-pills" role="group" aria-label="Interaction mode">
                    {MODES.map((mode) => (
                      <button
                        key={mode.value}
                        type="button"
                        className={
                          props.modePolicy.mode === mode.value
                            ? "rex-mode-pill rex-mode-pill--active"
                            : "rex-mode-pill"
                        }
                        aria-pressed={props.modePolicy.mode === mode.value}
                        onClick={() => props.onModeChange(mode.value)}
                      >
                        {mode.label}
                      </button>
                    ))}
                  </div>
                  <span className="rex-composer__hint" title={props.modePolicy.summary}>
                    {composerStatusHint({
                      streaming: props.streaming,
                      activityHint: props.activityHint,
                      canSend,
                      daemonReady: props.daemonReady,
                    })}
                  </span>
                  <span className="rex-composer__actions">
                    <button
                      type="button"
                      className="rex-icon-button"
                      onClick={props.onRequestContextPicker}
                      aria-label="Attach context"
                    >
                      @
                    </button>
                    {props.streaming ? (
                      <button
                        type="button"
                        className="rex-icon-button rex-icon-button--stop"
                        onClick={props.onCancel}
                        aria-label="Stop generation"
                      >
                        <StopIcon />
                      </button>
                    ) : (
                      <button
                        type="button"
                        className="rex-icon-button rex-icon-button--send"
                        onClick={props.onSubmit}
                        disabled={!canSend}
                        aria-label="Send prompt"
                      >
                        <SendIcon />
                      </button>
                    )}
                  </span>
                </div>
              </div>
            </div>
          </>
        ) : (
          <div
            id="rex-view-panel-plan"
            role="tabpanel"
            aria-labelledby="rex-view-tab-plan"
            className="rex-plan-panel"
          >
            <PlanCard
              panel
              artifact={props.planArtifact!}
              onContentChange={props.onPlanContentChange}
              onSavePathChange={props.onPlanSavePathChange}
              onSave={props.onPlanSave}
              onBuild={props.onPlanBuild}
            />
          </div>
        )}
      </div>
    </div>
  );
}

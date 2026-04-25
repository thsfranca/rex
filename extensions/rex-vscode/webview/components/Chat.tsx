import * as React from "react";

import type {
  ApprovalRequestPayload,
  ApplyGranularity,
  InteractionMode,
  ModePolicy,
  PromptContextSnapshot,
  ThemeKind,
} from "../../src/shared/messages";

import { Message, type RenderedMessage } from "./Message";

export interface ChatProps {
  readonly messages: ReadonlyArray<RenderedMessage>;
  readonly theme: ThemeKind;
  readonly context: PromptContextSnapshot | null;
  readonly attachContext: boolean;
  readonly streaming: boolean;
  readonly daemonReady: boolean;
  readonly modePolicy: ModePolicy;
  readonly timeline: ReadonlyArray<{ id: string; summary: string; phase: string }>;
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
}

export function Chat(props: ChatProps): React.ReactElement {
  const listRef = React.useRef<HTMLDivElement | null>(null);
  React.useEffect(() => {
    const el = listRef.current;
    if (el === null) {
      return;
    }
    el.scrollTop = el.scrollHeight;
  }, [props.messages]);

  const canSend = props.prompt.trim().length > 0 && !props.streaming && props.daemonReady;
  const canMutateFiles = props.modePolicy.canMutateFiles;

  const handleKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>): void => {
    if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
      event.preventDefault();
      if (canSend) {
        props.onSubmit();
      }
    }
  };

  return (
    <div className="rex-app" role="region" aria-label="REX Chat">
      <header className="rex-header">
        <div className="rex-header__status-wrap">
          <span className="rex-header__status" aria-live="polite">
            <span
              className={`rex-status-dot ${
                props.daemonReady ? "rex-status-dot--ready" : "rex-status-dot--unavailable"
              }`}
            />
            {props.daemonReady ? "Daemon ready" : "Daemon unavailable"}
          </span>
          <label className="rex-mode-select">
            Mode
            <select
              value={props.modePolicy.mode}
              onChange={(event) => props.onModeChange(event.target.value as InteractionMode)}
            >
              <option value="ask">Ask</option>
              <option value="plan">Plan</option>
              <option value="agent">Agent</option>
            </select>
          </label>
        </div>
        <span className="rex-header__actions">
          <button type="button" onClick={props.onClear} aria-label="Clear chat">
            Clear
          </button>
        </span>
      </header>
      <div className="rex-policy-note">{props.modePolicy.summary}</div>
      {props.pendingApprovals.map((approval) => (
        <div key={approval.id} className="rex-approval-card" role="alert">
          <div className="rex-approval-card__title">{approval.title}</div>
          <div className="rex-approval-card__detail">{approval.detail}</div>
          <div className="rex-approval-card__actions">
            <button type="button" onClick={() => props.onApprovalDecision(approval.id, true)}>
              Approve
            </button>
            <button type="button" onClick={() => props.onApprovalDecision(approval.id, false)}>
              Deny
            </button>
          </div>
        </div>
      ))}
      <div ref={listRef} className="rex-messages" role="log" aria-live="polite">
        {props.messages.length === 0 ? (
          <div className="rex-hint">
            Ask something about your code. Use `REX: Explain/Fix/Refactor Selection` from
            the editor to prefill a prompt.
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
      <div className="rex-composer">
        {props.context !== null ? (
          <div className="rex-context-chip" role="group" aria-label="Editor context">
            <label style={{ display: "inline-flex", gap: 4, alignItems: "center" }}>
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
        <textarea
          value={props.prompt}
          onChange={(event) => props.onPromptChange(event.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={`${props.modePolicy.mode.toUpperCase()} mode: Cmd/Ctrl+Enter to send`}
          aria-label="Prompt"
        />
        {props.timeline.length > 0 ? (
          <div className="rex-timeline" role="status" aria-live="polite">
            {props.timeline.map((entry) => (
              <div key={`${entry.id}-${entry.phase}-${entry.summary}`} className="rex-timeline__item">
                <span className="rex-timeline__phase">{entry.phase}</span>
                <span>{entry.summary}</span>
              </div>
            ))}
          </div>
        ) : null}
        <div className="rex-composer__row">
          <span className="rex-hint">
            {props.streaming ? "Streaming…" : canSend ? "Ready" : props.daemonReady ? "Type a prompt" : "Daemon unavailable"}
          </span>
          <span style={{ display: "inline-flex", gap: 6 }}>
            {props.streaming ? (
              <button
                type="button"
                className="rex-composer__cancel"
                onClick={props.onCancel}
                aria-label="Stop generation"
              >
                Stop
              </button>
            ) : null}
            <button
              type="button"
              className="rex-composer__send"
              onClick={props.onSubmit}
              disabled={!canSend}
              aria-label="Send prompt"
            >
              Send
            </button>
          </span>
        </div>
      </div>
    </div>
  );
}

import * as React from "react";

import type {
  ApplyGranularity,
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
  readonly prompt: string;
  readonly onPromptChange: (value: string) => void;
  readonly onAttachContextChange: (value: boolean) => void;
  readonly onSubmit: () => void;
  readonly onCancel: () => void;
  readonly onClear: () => void;
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
        <span className="rex-header__status" aria-live="polite">
          <span
            className={`rex-status-dot ${
              props.daemonReady ? "rex-status-dot--ready" : "rex-status-dot--unavailable"
            }`}
          />
          {props.daemonReady ? "Daemon ready" : "Daemon unavailable"}
        </span>
        <span className="rex-header__actions">
          <button type="button" onClick={props.onClear} aria-label="Clear chat">
            Clear
          </button>
        </span>
      </header>
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
          placeholder="Ask REX… (Cmd/Ctrl+Enter to send)"
          aria-label="Prompt"
        />
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

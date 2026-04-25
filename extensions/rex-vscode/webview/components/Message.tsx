import * as React from "react";

import type { ApplyGranularity, ApplyResultPayload, ThemeKind } from "../../src/shared/messages";
import { splitByCodeBlocks } from "../streaming/codeBlocks";
import { renderMarkdown } from "../streaming/markdownStream";

import { CodeBlock } from "./CodeBlock";

export type MessageRole = "user" | "assistant" | "system";

export interface RenderedMessage {
  readonly id: string;
  readonly role: MessageRole;
  readonly buffer: string;
  readonly trailingRaw: string;
  readonly streaming: boolean;
  readonly errorMessage?: string;
  readonly applyResults?: ReadonlyMap<string, ApplyResultPayload>;
}

export interface MessageProps {
  readonly message: RenderedMessage;
  readonly theme: ThemeKind;
  readonly onCopy: (code: string) => void;
  readonly onInsert: (code: string) => void;
  readonly onApply: (args: {
    id: string;
    language: string;
    code: string;
    granularity: ApplyGranularity;
  }) => void;
  readonly canMutateFiles: boolean;
}

export function Message(props: MessageProps): React.ReactElement {
  const { message, theme, onCopy, onInsert, onApply, canMutateFiles } = props;

  if (message.role === "user") {
    return (
      <div className="rex-message rex-message--user" aria-label="You said">
        <div className="rex-message__role">You</div>
        <div className="rex-message__body">{message.buffer}</div>
      </div>
    );
  }

  const segments = splitByCodeBlocks(message.buffer);
  return (
    <div className="rex-message rex-message--assistant" aria-label="Assistant reply">
      <div className="rex-message__role">REX</div>
      <div className="rex-message__body">
        {segments.map((segment, index) => {
          if (segment.kind === "markdown") {
            return (
              <div
                key={`md-${message.id}-${index}`}
                dangerouslySetInnerHTML={{ __html: renderMarkdown(segment.content) }}
              />
            );
          }
          const codeBlockId = `${message.id}-code-${index}`;
          const applyResult = message.applyResults?.get(codeBlockId);
          return (
            <CodeBlock
              key={codeBlockId}
              id={codeBlockId}
              language={segment.language ?? ""}
              code={segment.content}
              theme={theme}
              applyResult={applyResult}
              onCopy={onCopy}
              onInsert={onInsert}
              onApply={onApply}
              canMutateFiles={canMutateFiles}
            />
          );
        })}
        {message.trailingRaw.length > 0 ? (
          <span className="rex-trailing" aria-live="polite">
            {message.trailingRaw}
          </span>
        ) : null}
        {message.errorMessage !== undefined ? (
          <div className="rex-error" role="alert">
            {message.errorMessage}
          </div>
        ) : null}
      </div>
    </div>
  );
}


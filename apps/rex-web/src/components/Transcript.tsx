import { useRef, type ReactNode } from "react";
import { Virtuoso } from "react-virtuoso";
import ReactMarkdown from "react-markdown";
import type { Components } from "react-markdown";
import { Button, Text } from "../design-system";
import type { ChatMessage } from "../types";
import { MotionMessage } from "./Motion";

interface Props {
  messages: ChatMessage[];
}

function CodePre({ children }: { children?: ReactNode }) {
  const ref = useRef<HTMLPreElement>(null);
  return (
    <pre ref={ref}>
      <Button
        type="button"
        variant="ghost"
        className="message-block__copy"
        onClick={() => {
          void navigator.clipboard.writeText(ref.current?.textContent?.replace("Copy", "").trim() ?? "");
        }}
      >
        Copy
      </Button>
      {children}
    </pre>
  );
}

const markdownComponents: Components = {
  pre({ children }) {
    return <CodePre>{children}</CodePre>;
  },
};

export function Transcript({ messages }: Props) {
  if (messages.length === 0) {
    return (
      <div className="rex-transcript-empty" data-testid="transcript">
        <h2 className="rex-transcript-empty__title">What should we work on?</h2>
        <Text tone="secondary" className="rex-transcript-empty__hint">
          Ask Rex about your workspace. Use Agent mode for edits, Ask mode for questions.
        </Text>
      </div>
    );
  }

  return (
    <div data-testid="transcript">
      <Virtuoso
        data={messages}
        followOutput="smooth"
        skipAnimationFrameInResizeObserver
        itemContent={(_index, message) => {
          const isUser = message.role === "user";
          return (
            <div className={`rex-message-row rex-message-row--${isUser ? "user" : "assistant"}`}>
              <MotionMessage
                className={`rex-message-bubble rex-message-bubble--${isUser ? "user" : "assistant"} message-block`}
              >
                <span className="rex-message-bubble__role">{isUser ? "You" : "Rex"}</span>
                {isUser ? (
                  message.content
                ) : (
                  <ReactMarkdown components={markdownComponents}>{message.content}</ReactMarkdown>
                )}
              </MotionMessage>
            </div>
          );
        }}
      />
    </div>
  );
}

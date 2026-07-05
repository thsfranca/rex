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
      <div data-testid="transcript">
        <Text tone="secondary">Ask Rex anything about your workspace.</Text>
      </div>
    );
  }

  return (
    <div data-testid="transcript">
      <Virtuoso
        data={messages}
        followOutput="smooth"
        skipAnimationFrameInResizeObserver
        itemContent={(_index, message) => (
          <MotionMessage
            className={`message-block ${message.role === "user" ? "message-user" : "message-assistant"}`}
          >
            {message.role === "assistant" ? (
              <ReactMarkdown components={markdownComponents}>{message.content}</ReactMarkdown>
            ) : (
              message.content
            )}
          </MotionMessage>
        )}
      />
    </div>
  );
}

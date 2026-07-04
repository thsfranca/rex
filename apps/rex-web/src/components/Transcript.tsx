import { Virtuoso } from "react-virtuoso";
import ReactMarkdown from "react-markdown";
import type { ChatMessage } from "../types";

interface Props {
  messages: ChatMessage[];
}

export function Transcript({ messages }: Props) {
  if (messages.length === 0) {
    return (
      <div className="transcript" data-testid="transcript">
        <p style={{ color: "var(--rex-text-secondary)" }}>
          Ask Rex anything about your workspace.
        </p>
      </div>
    );
  }

  return (
    <div className="transcript" data-testid="transcript">
      <Virtuoso
        data={messages}
        followOutput="smooth"
        skipAnimationFrameInResizeObserver
        itemContent={(_index, message) => (
          <div
            className={
              message.role === "user" ? "message-user" : "message-assistant"
            }
          >
            {message.role === "assistant" ? (
              <ReactMarkdown>{message.content}</ReactMarkdown>
            ) : (
              message.content
            )}
          </div>
        )}
      />
    </div>
  );
}

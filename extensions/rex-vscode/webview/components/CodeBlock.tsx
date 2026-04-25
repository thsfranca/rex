import * as React from "react";

import type { ApplyGranularity, ApplyResultPayload, ThemeKind } from "../../src/shared/messages";
import { highlight } from "../streaming/highlight";

export interface CodeBlockProps {
  readonly id: string;
  readonly language: string;
  readonly code: string;
  readonly theme: ThemeKind;
  readonly applyResult: ApplyResultPayload | undefined;
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

const STATUS_LABEL: Record<string, string> = {
  applied: "Applied",
  rejected: "Rejected",
  cancelled: "Cancelled",
  error: "Error",
};

export function CodeBlock(props: CodeBlockProps): React.ReactElement {
  const { id, language, code, theme, applyResult, onCopy, onInsert, onApply, canMutateFiles } = props;
  const [highlighted, setHighlighted] = React.useState<string | undefined>(undefined);
  const [copied, setCopied] = React.useState(false);

  React.useEffect(() => {
    let cancelled = false;
    void highlight(code, language || "text", theme).then((html) => {
      if (!cancelled) {
        setHighlighted(html);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [code, language, theme]);

  const handleCopy = (): void => {
    onCopy(code);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className="rex-codeblock" role="group" aria-label={`Code block (${language || "text"})`}>
      <div className="rex-codeblock__header">
        <span className="rex-codeblock__lang">{language || "text"}</span>
        <div className="rex-codeblock__actions">
          <button type="button" onClick={handleCopy} aria-label="Copy code">
            {copied ? "Copied" : "Copy"}
          </button>
          <button type="button" onClick={() => onInsert(code)} aria-label="Insert at cursor" disabled={!canMutateFiles}>
            Insert
          </button>
          <button
            type="button"
            onClick={() =>
              onApply({ id, language, code, granularity: "selection" })
            }
            aria-label="Apply to selection"
            disabled={!canMutateFiles}
          >
            Apply
          </button>
        </div>
      </div>
      <div className="rex-codeblock__body">
        {highlighted !== undefined ? (
          <div dangerouslySetInnerHTML={{ __html: highlighted }} />
        ) : (
          <pre>
            <code>{code}</code>
          </pre>
        )}
      </div>
      {applyResult !== undefined ? (
        <div
          className={`rex-codeblock__status rex-codeblock__status--${applyResult.outcome}`}
          role="status"
        >
          {STATUS_LABEL[applyResult.outcome] ?? applyResult.outcome}
          {applyResult.detail !== undefined ? `: ${applyResult.detail}` : ""}
        </div>
      ) : null}
    </div>
  );
}

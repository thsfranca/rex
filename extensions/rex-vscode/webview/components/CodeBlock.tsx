import * as React from "react";

import type { ApplyGranularity, ApplyResultPayload, ThemeKind } from "../../src/shared/messages";
import { highlight } from "../streaming/highlight";

/** Lines above this show collapsed preview + expand control (Open WebUI / EUI pattern). */
const COLLAPSE_LINE_THRESHOLD = 14;

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

function countLines(code: string): number {
  if (code.length === 0) {
    return 0;
  }
  return code.split("\n").length;
}

export function CodeBlock(props: CodeBlockProps): React.ReactElement {
  const { id, language, code, theme, applyResult, onCopy, onInsert, onApply, canMutateFiles } = props;
  const [highlighted, setHighlighted] = React.useState<string | undefined>(undefined);
  const [copied, setCopied] = React.useState(false);
  const [expanded, setExpanded] = React.useState(false);

  const lineCount = countLines(code);
  const isCollapsible = lineCount > COLLAPSE_LINE_THRESHOLD;
  const isCollapsed = isCollapsible && !expanded;
  const langLabel = language || "text";

  React.useEffect(() => {
    let cancelled = false;
    void highlight(code, langLabel, theme).then((html) => {
      if (!cancelled) {
        setHighlighted(html);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [code, langLabel, theme]);

  React.useEffect(() => {
    setExpanded(false);
  }, [code]);

  const handleCopy = (): void => {
    onCopy(code);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1500);
  };

  const scrollClassName = isCollapsed
    ? "rex-codeblock__scroll rex-codeblock__scroll--collapsed"
    : isCollapsible
      ? "rex-codeblock__scroll rex-codeblock__scroll--expanded"
      : "rex-codeblock__scroll";

  return (
    <div className="rex-codeblock" role="group" aria-label={`Code block (${langLabel})`}>
      <div className="rex-codeblock__header">
        <span className="rex-codeblock__lang">{langLabel}</span>
        <div className="rex-codeblock__actions">
          {isCollapsible ? (
            <button
              type="button"
              onClick={() => setExpanded((value) => !value)}
              aria-expanded={!isCollapsed}
              aria-label={isCollapsed ? `Show all ${lineCount} lines` : "Collapse code block"}
            >
              {isCollapsed ? `Show all (${lineCount})` : "Collapse"}
            </button>
          ) : null}
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
      <div
        className={scrollClassName}
        tabIndex={0}
        role="region"
        aria-label={`${langLabel} code, scroll horizontally for long lines`}
      >
        {highlighted !== undefined ? (
          <div className="rex-codeblock__highlight" dangerouslySetInnerHTML={{ __html: highlighted }} />
        ) : (
          <pre>
            <code>{code}</code>
          </pre>
        )}
      </div>
      {isCollapsed ? (
        <button
          type="button"
          className="rex-codeblock__expand-bar"
          onClick={() => setExpanded(true)}
          aria-label={`Show all ${lineCount} lines of code`}
        >
          {lineCount - COLLAPSE_LINE_THRESHOLD} more lines — expand
        </button>
      ) : null}
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

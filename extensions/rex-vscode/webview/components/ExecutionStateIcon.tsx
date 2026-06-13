import * as React from "react";

export type ExecutionPhase =
  | "queued"
  | "running"
  | "awaiting_approval"
  | "completed"
  | "blocked"
  | "failed"
  | "cancelled"
  | string;

export interface ExecutionStateIconProps {
  readonly phase: ExecutionPhase;
}

export function ExecutionStateIcon(props: ExecutionStateIconProps): React.ReactElement {
  switch (props.phase) {
    case "running":
      return (
        <svg className="rex-exec-icon rex-exec-icon--spin" width="14" height="14" viewBox="0 0 16 16" aria-hidden="true">
          <circle cx="8" cy="8" r="6" stroke="currentColor" strokeWidth="1.5" fill="none" strokeDasharray="28" strokeDashoffset="8" />
        </svg>
      );
    case "completed":
      return (
        <svg className="rex-exec-icon rex-exec-icon--done" width="14" height="14" viewBox="0 0 16 16" aria-hidden="true">
          <path
            d="M8 1.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13Zm2.8 4.7-3.5 3.5a.6.6 0 0 1-.85 0L5.2 8.5a.6.6 0 1 1 .85-.85L7 8.8l3.08-3.08a.6.6 0 1 1 .85.85Z"
            fill="currentColor"
          />
        </svg>
      );
    case "failed":
      return (
        <svg className="rex-exec-icon rex-exec-icon--failed" width="14" height="14" viewBox="0 0 16 16" aria-hidden="true">
          <path
            d="M8 1.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13Zm1.1 3.4a.6.6 0 0 0-.85 0L8 6.15 6.75 4.9a.6.6 0 1 0-.85.85L7.15 7l-1.25 1.25a.6.6 0 1 0 .85.85L8 7.85l1.25 1.25a.6.6 0 0 0 .85-.85L8.85 7l1.25-1.25a.6.6 0 0 0-.85-.85Z"
            fill="currentColor"
          />
        </svg>
      );
    case "cancelled":
      return (
        <svg className="rex-exec-icon rex-exec-icon--cancelled" width="14" height="14" viewBox="0 0 16 16" aria-hidden="true">
          <path d="M4 8h8" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
        </svg>
      );
    case "awaiting_approval":
      return (
        <svg className="rex-exec-icon rex-exec-icon--approval" width="14" height="14" viewBox="0 0 16 16" aria-hidden="true">
          <path
            d="M8 1.8 3.2 3.6v3.9c0 2.8 2 5.4 4.8 6.2 2.8-.8 4.8-3.4 4.8-6.2V3.6L8 1.8Zm0 2.2 2.5 1v2.5c0 1.6-1.1 3.1-2.5 3.6-1.4-.5-2.5-2-2.5-3.6V5l2.5-1Z"
            fill="currentColor"
          />
        </svg>
      );
    case "blocked":
      return (
        <svg className="rex-exec-icon rex-exec-icon--blocked" width="14" height="14" viewBox="0 0 16 16" aria-hidden="true">
          <path
            d="M8 1.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13Zm-1.2 3h2.4v5.2H6.8V4.5Zm0 6.2h2.4V12H6.8v-1.3Z"
            fill="currentColor"
          />
        </svg>
      );
    case "queued":
    default:
      return (
        <svg className="rex-exec-icon rex-exec-icon--queued" width="14" height="14" viewBox="0 0 16 16" aria-hidden="true">
          <circle cx="8" cy="8" r="6" stroke="currentColor" strokeWidth="1.5" fill="none" opacity="0.55" />
        </svg>
      );
  }
}

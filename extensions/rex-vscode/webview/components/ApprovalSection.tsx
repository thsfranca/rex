import * as React from "react";

import type { ApprovalRequestPayload, ApprovalScope } from "../../src/shared/messages";

import { EditDiffPreview } from "./EditDiffPreview";

const SCOPE_LABEL: Record<ApprovalScope, string> = {
  mutation: "Workspace edit",
  execution: "Tool execution",
};

export interface ApprovalSectionProps {
  readonly approvals: ReadonlyArray<ApprovalRequestPayload>;
  readonly onDecision: (id: string, approved: boolean) => void;
}

export function ApprovalSection(props: ApprovalSectionProps): React.ReactElement | null {
  if (props.approvals.length === 0) {
    return null;
  }

  const countLabel =
    props.approvals.length === 1 ? "1 pending approval" : `${props.approvals.length} pending approvals`;

  return (
    <section className="rex-approval-section" aria-label="Workspace edit approvals">
      <header className="rex-approval-section__header">
        <div className="rex-approval-section__heading">
          <h2 className="rex-approval-section__title">Approvals required</h2>
          <p className="rex-approval-section__subtitle">Review proposed changes before they apply</p>
        </div>
        <span className="rex-approval-section__count" aria-live="polite">
          {countLabel}
        </span>
      </header>
      <ul className="rex-approval-section__list">
        {props.approvals.map((approval) => {
          const hasDiff = approval.edits !== undefined && approval.edits.length > 0;
          return (
            <li
              key={approval.id}
              className={hasDiff ? "rex-approval-card rex-approval-card--with-diff" : "rex-approval-card"}
            >
              <div className="rex-approval-card__meta">
                <span className="rex-approval-card__scope">{SCOPE_LABEL[approval.scope]}</span>
                <div className="rex-approval-card__title">{approval.title}</div>
                <div className="rex-approval-card__detail">{approval.detail}</div>
              </div>
              {hasDiff ? (
                <div className="rex-approval-card__diffs">
                  {approval.edits!.map((edit, index) => (
                    <EditDiffPreview key={`${approval.id}-${edit.filePath}-${index}`} edit={edit} />
                  ))}
                </div>
              ) : null}
              <div className="rex-approval-card__actions">
                <button type="button" onClick={() => props.onDecision(approval.id, false)}>
                  Deny
                </button>
                <button
                  type="button"
                  className="rex-approval-card__approve"
                  onClick={() => props.onDecision(approval.id, true)}
                >
                  Approve
                </button>
              </div>
            </li>
          );
        })}
      </ul>
    </section>
  );
}

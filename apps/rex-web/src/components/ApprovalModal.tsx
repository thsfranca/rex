import { motion, useReducedMotion } from "framer-motion";
import type { PendingApproval } from "../types";

interface Props {
  pending: PendingApproval;
  onApprove: () => void;
  onDeny: () => void;
}

export function ApprovalModal({ pending, onApprove, onDeny }: Props) {
  const reduceMotion = useReducedMotion();
  const lines = pending.detail.split("\n");

  return (
    <div
      className="modal-backdrop open"
      data-testid="modal-backdrop"
      role="dialog"
      aria-modal="true"
      aria-labelledby="approval-title"
    >
      <motion.div
        className="modal"
        data-testid="modal"
        data-motion-tier="cinematic"
        initial={reduceMotion ? false : { opacity: 0, scale: 0.95 }}
        animate={{ opacity: 1, scale: 1 }}
        exit={reduceMotion ? undefined : { opacity: 0, scale: 0.95 }}
        transition={{ duration: 0.35, ease: [0.25, 1, 0.5, 1] }}
      >
        <h2 id="approval-title" style={{ marginTop: 0 }}>
          {pending.toolName} needs your approval
        </h2>
        <p style={{ color: "var(--rex-text-secondary)" }}>
          Review the proposed change before Rex continues.
        </p>
        <div className="diff-scrubber" data-testid="diff-scrubber">
          {lines.map((line, index) => (
            <div key={index} className="diff-line">
              <span className="diff-marker">+</span>
              <code>{line || pending.detail}</code>
            </div>
          ))}
        </div>
        <div className="modal-actions">
          <button type="button" data-testid="approval-deny" onClick={onDeny}>
            Deny
          </button>
          <button
            type="button"
            data-testid="approval-approve"
            className="primary"
            onClick={onApprove}
          >
            Approve
          </button>
        </div>
      </motion.div>
    </div>
  );
}

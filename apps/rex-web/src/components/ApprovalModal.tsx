import { motion, useReducedMotion } from "framer-motion";
import { Button } from "../design-system";
import { modalTransition, modalVariants } from "../design-system/motion";
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
        initial={reduceMotion ? false : "hidden"}
        animate="visible"
        exit={reduceMotion ? undefined : "hidden"}
        variants={modalVariants}
        transition={modalTransition}
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
          <Button type="button" variant="secondary" data-testid="approval-deny" onClick={onDeny}>
            Deny
          </Button>
          <Button type="button" variant="primary" data-testid="approval-approve" onClick={onApprove}>
            Approve
          </Button>
        </div>
      </motion.div>
    </div>
  );
}

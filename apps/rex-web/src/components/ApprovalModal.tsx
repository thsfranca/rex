import { Button, AnimatedModal } from "../design-system";
import { ModalParticleRing } from "./ModalParticleRing";
import type { PendingApproval } from "../types";

interface Props {
  open: boolean;
  pending: PendingApproval;
  onApprove: () => void;
  onDeny: () => void;
}

export function ApprovalModal({ open, pending, onApprove, onDeny }: Props) {
  const lines = pending.detail.split("\n");

  return (
    <>
      <ModalParticleRing active={open} />
      <AnimatedModal
        open={open}
        title={`${pending.toolName} needs your approval`}
        testId="modal-backdrop"
        actions={
          <>
            <Button type="button" variant="secondary" data-testid="approval-deny" onClick={onDeny}>
              Deny
            </Button>
            <Button type="button" variant="primary" data-testid="approval-approve" onClick={onApprove}>
              Approve
            </Button>
          </>
        }
      >
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
      </AnimatedModal>
    </>
  );
}

import * as React from "react";

import type { ContinueTurnPayload } from "../../src/shared/messages";

export interface ContinueSectionProps {
  readonly pending: ContinueTurnPayload | null;
  readonly onDecision: (streamId: string, continueToken: string, continueTurn: boolean) => void;
}

export function ContinueSection(props: ContinueSectionProps): React.ReactElement | null {
  if (props.pending === null) {
    return null;
  }
  return (
    <section className="rex-continue-section" aria-label="Step budget pause">
      <p>{props.pending.summary}</p>
      <div className="rex-continue-actions">
        <button
          type="button"
          className="rex-btn rex-btn--primary"
          onClick={() =>
            props.onDecision(props.pending!.streamId, props.pending!.continueToken, true)
          }
        >
          Continue
        </button>
        <button
          type="button"
          className="rex-btn"
          onClick={() =>
            props.onDecision(props.pending!.streamId, props.pending!.continueToken, false)
          }
        >
          Stop
        </button>
      </div>
    </section>
  );
}

import { useState } from "react";
import { MotionSessionCard } from "../design-system";
import type { SessionSummary } from "../types";

interface Props {
  sessions: SessionSummary[];
  onSelect: (sessionId: string) => void;
  onClose: () => void;
}

export function SessionPicker({ sessions, onSelect, onClose }: Props) {
  const [focusedId] = useState<string | null>(sessions[0]?.id ?? null);

  return (
    <div className="modal-backdrop open rex-modal-backdrop--dim" data-testid="session-picker-backdrop">
      <div className="modal session-picker" data-testid="session-picker" data-motion-tier="cinematic">
        <h2 style={{ marginTop: 0 }}>Sessions</h2>
        <div className="session-carousel" data-testid="session-carousel">
          {sessions.length === 0 ? (
            <p style={{ color: "var(--rex-text-secondary)" }}>No prior sessions yet.</p>
          ) : (
            sessions.map((session) => (
              <MotionSessionCard
                key={session.id}
                className="session-card"
                data-testid={`session-card-${session.id}`}
                focused={focusedId === session.id}
                onClick={() => onSelect(session.id)}
              >
                <strong>{session.title}</strong>
                <span>{session.preview}</span>
              </MotionSessionCard>
            ))
          )}
        </div>
        <button type="button" onClick={onClose}>
          Close
        </button>
      </div>
    </div>
  );
}

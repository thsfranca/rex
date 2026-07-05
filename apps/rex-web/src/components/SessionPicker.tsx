import type { SessionSummary } from "../types";

interface Props {
  sessions: SessionSummary[];
  onSelect: (sessionId: string) => void;
  onClose: () => void;
}

export function SessionPicker({ sessions, onSelect, onClose }: Props) {
  return (
    <div className="modal-backdrop open" data-testid="session-picker-backdrop">
      <div className="modal session-picker" data-testid="session-picker">
        <h2 style={{ marginTop: 0 }}>Sessions</h2>
        <div className="session-carousel" data-testid="session-carousel">
          {sessions.length === 0 ? (
            <p style={{ color: "var(--rex-text-secondary)" }}>No prior sessions yet.</p>
          ) : (
            sessions.map((session) => (
              <button
                key={session.id}
                type="button"
                className="session-card"
                data-testid={`session-card-${session.id}`}
                onClick={() => onSelect(session.id)}
              >
                <strong>{session.title}</strong>
                <span>{session.preview}</span>
              </button>
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

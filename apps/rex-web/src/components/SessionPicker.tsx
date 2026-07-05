import { useEffect, useRef, useState } from "react";
import { AnimatedModal, MotionSessionCard } from "../design-system";
import type { SessionSummary } from "../types";

interface Props {
  sessions: SessionSummary[];
  onSelect: (sessionId: string) => void;
  onClose: () => void;
}

export function SessionPicker({ sessions, onSelect, onClose }: Props) {
  const [focusedId, setFocusedId] = useState<string | null>(sessions[0]?.id ?? null);
  const carouselRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const root = carouselRef.current;
    if (!root) return;
    const cards = root.querySelectorAll<HTMLElement>("[data-session-id]");
    const observer = new IntersectionObserver(
      (entries) => {
        const visible = entries
          .filter((entry) => entry.isIntersecting)
          .sort((a, b) => b.intersectionRatio - a.intersectionRatio)[0];
        if (visible?.target instanceof HTMLElement) {
          setFocusedId(visible.target.dataset.sessionId ?? null);
        }
      },
      { root, threshold: [0.55, 0.75, 0.95] },
    );
    cards.forEach((card) => observer.observe(card));
    return () => observer.disconnect();
  }, [sessions]);

  return (
    <AnimatedModal
      open
      title="Sessions"
      testId="session-picker-backdrop"
      modalClassName="rex-modal modal session-picker"
      onClose={onClose}
    >
      <div className="session-carousel" data-testid="session-carousel" ref={carouselRef}>
        {sessions.length === 0 ? (
          <p style={{ color: "var(--rex-text-secondary)" }}>No prior sessions yet.</p>
        ) : (
          sessions.map((session) => (
            <MotionSessionCard
              key={session.id}
              className="session-card"
              data-testid={`session-card-${session.id}`}
              data-session-id={session.id}
              focused={focusedId === session.id}
              onClick={() => onSelect(session.id)}
            >
              <strong>{session.title}</strong>
              <span>{session.preview}</span>
            </MotionSessionCard>
          ))
        )}
      </div>
    </AnimatedModal>
  );
}

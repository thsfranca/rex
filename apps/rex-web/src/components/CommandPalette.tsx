import { useEffect, useMemo, useState, type CSSProperties } from "react";
import { Button, MotionBanner } from "../design-system";
import { ERROR_HSL, NEUTRAL_HSL, lerpHslCss } from "../design-system/physics/hsl-lerp";
import { useSpringScalar } from "../design-system/motion/useSpringScalar";

export interface CommandAction {
  id: string;
  label: string;
  shortcut?: string;
  run: () => void;
}

interface Props {
  open: boolean;
  actions: CommandAction[];
  onClose: () => void;
}

export function CommandPalette({ open, actions, onClose }: Props) {
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return actions;
    return actions.filter((action) => action.label.toLowerCase().includes(q));
  }, [actions, query]);

  useEffect(() => {
    if (!open) {
      setQuery("");
      setActiveIndex(0);
    }
  }, [open]);

  useEffect(() => {
    setActiveIndex(0);
  }, [query]);

  useEffect(() => {
    if (!open) return;
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setActiveIndex((i) => Math.min(i + 1, filtered.length - 1));
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setActiveIndex((i) => Math.max(i - 1, 0));
      }
      if (e.key === "Enter" && filtered[activeIndex]) {
        e.preventDefault();
        filtered[activeIndex].run();
        onClose();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, filtered, activeIndex, onClose]);

  if (!open) return null;

  return (
    <div
      className="rex-command-palette-backdrop"
      data-testid="command-palette-backdrop"
      role="presentation"
      onClick={onClose}
    >
      <div
        className="rex-command-palette"
        data-testid="command-palette"
        role="dialog"
        aria-modal="true"
        aria-label="Command palette"
        onClick={(e) => e.stopPropagation()}
      >
        <input
          className="rex-command-palette__input"
          data-testid="command-palette-input"
          placeholder="Search commands…"
          value={query}
          autoFocus
          onChange={(e) => setQuery(e.target.value)}
        />
        <ul className="rex-command-palette__list">
          {filtered.map((action, index) => (
            <li key={action.id}>
              <button
                type="button"
                className={`rex-command-palette__item${index === activeIndex ? " rex-command-palette__item--active" : ""}`}
                data-testid={`command-${action.id}`}
                onClick={() => {
                  action.run();
                  onClose();
                }}
              >
                {action.label}
                {action.shortcut ? ` · ${action.shortcut}` : ""}
              </button>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}

export function ErrorBanner({
  message,
  onDismiss,
}: {
  message: string;
  onDismiss: () => void;
}) {
  const wash = useSpringScalar(1, { stiffness: 220, damping: 18 });
  const bannerStyle = {
    "--rex-banner-wash": lerpHslCss(NEUTRAL_HSL, ERROR_HSL, wash),
  } as CSSProperties;

  return (
    <MotionBanner className="rex-banner rex-banner--error" testId="error-banner" style={bannerStyle}>
      <span className="rex-banner__message">{message}</span>
      <Button type="button" variant="ghost" data-testid="error-banner-dismiss" onClick={onDismiss}>
        Dismiss
      </Button>
    </MotionBanner>
  );
}

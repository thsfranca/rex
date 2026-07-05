import { useEffect, useRef, type ReactNode } from "react";

export interface ModalProps {
  open: boolean;
  onClose?: () => void;
  title?: string;
  children: ReactNode;
  testId?: string;
  actions?: ReactNode;
}

export function Modal({ open, onClose, title, children, testId, actions }: ModalProps) {
  const backdropRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") onClose?.();
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      ref={backdropRef}
      className={`rex-modal-backdrop${open ? " rex-modal-backdrop--open" : ""}`}
      data-testid={testId}
      role="presentation"
      onClick={(e) => {
        if (e.target === backdropRef.current) onClose?.();
      }}
    >
      <div
        className="rex-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby={title ? "rex-modal-title" : undefined}
      >
        {title ? (
          <h2 id="rex-modal-title" className="rex-modal__title">
            {title}
          </h2>
        ) : null}
        <div className="rex-modal__body">{children}</div>
        {actions ? <div className="rex-modal__actions">{actions}</div> : null}
      </div>
    </div>
  );
}

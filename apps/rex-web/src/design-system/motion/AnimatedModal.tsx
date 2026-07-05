import { AnimatePresence, motion, useReducedMotion } from "framer-motion";
import type { ReactNode } from "react";
import { modalSpringTransition, modalVariants } from "./presets";

export interface AnimatedModalProps {
  open: boolean;
  onClose?: () => void;
  title?: string;
  children: ReactNode;
  actions?: ReactNode;
  testId?: string;
  backdropClassName?: string;
  modalClassName?: string;
  overlay?: ReactNode;
}

export function AnimatedModal({
  open,
  onClose,
  title,
  children,
  actions,
  testId = "modal-backdrop",
  backdropClassName = "rex-modal-backdrop rex-modal-backdrop--open rex-modal-backdrop--dim",
  modalClassName = "rex-modal modal",
  overlay,
}: AnimatedModalProps) {
  const reduceMotion = useReducedMotion();

  return (
    <AnimatePresence>
      {open ? (
        <motion.div
          className={backdropClassName}
          data-testid={testId}
          role="presentation"
          initial={reduceMotion ? false : { opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={reduceMotion ? undefined : { opacity: 0, transition: { duration: 0.25 } }}
          onClick={(e) => {
            if (e.target === e.currentTarget) onClose?.();
          }}
        >
          {overlay}
          <motion.div
            className={modalClassName}
            data-testid="modal"
            data-motion-tier="cinematic"
            role="dialog"
            aria-modal="true"
            aria-labelledby={title ? "rex-modal-title" : undefined}
            initial={reduceMotion ? false : "hidden"}
            animate="visible"
            exit={reduceMotion ? undefined : "hidden"}
            variants={modalVariants}
            transition={modalSpringTransition()}
            onClick={(e) => e.stopPropagation()}
          >
            {title ? (
              <h2 id="rex-modal-title" className="rex-modal__title">
                {title}
              </h2>
            ) : null}
            <div className="rex-modal__body">{children}</div>
            {actions ? <div className="rex-modal__actions modal-actions">{actions}</div> : null}
          </motion.div>
        </motion.div>
      ) : null}
    </AnimatePresence>
  );
}

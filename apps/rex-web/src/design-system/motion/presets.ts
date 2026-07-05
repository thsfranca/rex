import type { Transition, Variants } from "framer-motion";

export const ambientEase = [0.33, 1, 0.68, 1] as const;
export const activeEase = [0.25, 1, 0.5, 1] as const;

export const ambientTransition: Transition = {
  duration: 0.4,
  ease: ambientEase,
};

export const activeTransition: Transition = {
  duration: 0.25,
  ease: activeEase,
};

export const modalTransition: Transition = {
  duration: 0.35,
  ease: activeEase,
};

export const messageVariants: Variants = {
  hidden: { opacity: 0, y: 8 },
  visible: { opacity: 1, y: 0 },
};

export const timelineItemVariants: Variants = {
  hidden: { opacity: 0, x: 8 },
  visible: { opacity: 1, x: 0 },
};

export const modalVariants: Variants = {
  hidden: { opacity: 0, scale: 0.95 },
  visible: { opacity: 1, scale: 1 },
};

export const statusPulseTransition: Transition = {
  duration: 1.2,
  repeat: Infinity,
  ease: "easeInOut",
};

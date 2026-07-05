import type { Transition, Variants } from "framer-motion";
import { readSpringTokens } from "../physics/rk4-spring";

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

export function modalSpringTransition(): Transition {
  const { stiffness, damping } = readSpringTokens();
  return {
    type: "spring",
    stiffness,
    damping,
    mass: 1,
  };
}

export const modalTransition: Transition = modalSpringTransition();

export const messageVariants: Variants = {
  hidden: { opacity: 0, y: 12, filter: "blur(4px)" },
  visible: { opacity: 1, y: 0, filter: "blur(0px)" },
};

export const timelineItemVariants: Variants = {
  hidden: { opacity: 0, x: 12, scale: 0.96 },
  visible: { opacity: 1, x: 0, scale: 1 },
};

export const modalVariants: Variants = {
  hidden: { opacity: 0, scale: 0.92, y: 8 },
  visible: { opacity: 1, scale: 1, y: 0 },
};

export const bannerVariants: Variants = {
  hidden: { opacity: 0, y: -24, scale: 0.98 },
  visible: { opacity: 1, y: 0, scale: 1 },
};

export const statusPulseTransition: Transition = {
  duration: 1.2,
  repeat: Infinity,
  ease: "easeInOut",
};

export const staggerChildren = 0.06;

export const pressSpring: Transition = {
  type: "spring",
  stiffness: 400,
  damping: 28,
  mass: 0.8,
};

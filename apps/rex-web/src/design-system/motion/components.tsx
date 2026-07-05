import { motion, useReducedMotion } from "framer-motion";
import type { ReactNode } from "react";
import { pressSpring } from "./presets";

interface MotionPressableProps {
  children: ReactNode;
  className?: string;
  disabled?: boolean;
  onClick?: () => void;
  type?: "button" | "submit" | "reset";
  "data-testid"?: string;
}

export function MotionPressable({
  children,
  className,
  disabled,
  onClick,
  type = "button",
  "data-testid": testId,
}: MotionPressableProps) {
  const reduceMotion = useReducedMotion();

  if (reduceMotion) {
    return (
      <button
        type={type}
        className={className}
        disabled={disabled}
        onClick={onClick}
        data-testid={testId}
      >
        {children}
      </button>
    );
  }

  return (
    <motion.button
      type={type}
      className={className}
      disabled={disabled}
      onClick={onClick}
      data-testid={testId}
      data-motion-tier="active"
      whileHover={{ scale: 1.02, boxShadow: "var(--rex-shadow-glow-working)" }}
      whileTap={{ scale: 0.97 }}
      transition={pressSpring}
    >
      {children}
    </motion.button>
  );
}

interface MotionBannerProps {
  children: ReactNode;
  className?: string;
  testId?: string;
  role?: string;
}

export function MotionBanner({ children, className, testId, role = "alert" }: MotionBannerProps) {
  const reduceMotion = useReducedMotion();

  if (reduceMotion) {
    return (
      <div className={className} data-testid={testId} role={role}>
        {children}
      </div>
    );
  }

  return (
    <motion.div
      className={className}
      data-testid={testId}
      role={role}
      data-motion-tier="cinematic"
      initial={{ opacity: 0, y: -24, scale: 0.98 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, y: -12 }}
      transition={{ type: "spring", stiffness: 320, damping: 24 }}
    >
      {children}
    </motion.div>
  );
}

interface MotionSessionCardProps {
  children: ReactNode;
  className?: string;
  onClick?: () => void;
  focused?: boolean;
  "data-testid"?: string;
}

export function MotionSessionCard({
  children,
  className,
  onClick,
  focused = false,
  "data-testid": testId,
}: MotionSessionCardProps) {
  const reduceMotion = useReducedMotion();

  if (reduceMotion) {
    return (
      <button type="button" className={className} data-testid={testId} onClick={onClick}>
        {children}
      </button>
    );
  }

  return (
    <motion.button
      type="button"
      className={className}
      data-testid={testId}
      data-motion-tier="active"
      onClick={onClick}
      animate={{ scale: focused ? 1.04 : 1, opacity: focused ? 1 : 0.88 }}
      whileHover={{ scale: 1.03, opacity: 1 }}
      transition={{ type: "spring", stiffness: 280, damping: 22 }}
    >
      {children}
    </motion.button>
  );
}

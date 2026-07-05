import { motion, useReducedMotion } from "framer-motion";
import type { ButtonHTMLAttributes, ReactNode } from "react";
import { pressSpring } from "../motion/presets";

type Variant = "primary" | "secondary" | "ghost" | "danger";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  children: ReactNode;
  "data-testid"?: string;
}

const variantClass: Record<Variant, string> = {
  primary: "rex-btn rex-btn--primary",
  secondary: "rex-btn rex-btn--secondary",
  ghost: "rex-btn rex-btn--ghost",
  danger: "rex-btn rex-btn--danger",
};

export function Button({
  variant = "primary",
  className,
  type = "button",
  children,
  disabled,
  onClick,
  "data-testid": testId,
  id,
  form,
  name,
  value,
  autoFocus,
  tabIndex,
  "aria-label": ariaLabel,
  "aria-disabled": ariaDisabled,
}: ButtonProps) {
  const reduceMotion = useReducedMotion();
  const classes = className ? `${variantClass[variant]} ${className}` : variantClass[variant];
  const shared = {
    type,
    className: classes,
    disabled,
    onClick,
    "data-testid": testId,
    id,
    form,
    name,
    value,
    autoFocus,
    tabIndex,
    "aria-label": ariaLabel,
    "aria-disabled": ariaDisabled,
  };

  if (reduceMotion || disabled) {
    return <button {...shared}>{children}</button>;
  }

  return (
    <motion.button
      {...shared}
      data-motion-tier="active"
      whileHover={{ scale: 1.02 }}
      whileTap={{ scale: 0.97 }}
      transition={pressSpring}
    >
      {children}
    </motion.button>
  );
}

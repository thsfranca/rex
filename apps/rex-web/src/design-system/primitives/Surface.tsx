import { motion, useReducedMotion } from "framer-motion";
import type { HTMLAttributes, ReactNode } from "react";
import { pressSpring } from "../motion/presets";

type Elevation = "base" | "raised" | "overlay";

export interface SurfaceProps extends HTMLAttributes<HTMLDivElement> {
  elevation?: Elevation;
  children: ReactNode;
  interactive?: boolean;
  "data-testid"?: string;
}

const elevationClass: Record<Elevation, string> = {
  base: "rex-surface rex-surface--base",
  raised: "rex-surface rex-surface--raised",
  overlay: "rex-surface rex-surface--overlay",
};

export function Surface({
  elevation = "raised",
  className,
  children,
  interactive = false,
  onClick,
  "data-testid": testId,
  id,
  role,
  tabIndex,
  "aria-label": ariaLabel,
}: SurfaceProps) {
  const reduceMotion = useReducedMotion();
  const classes = className
    ? `${elevationClass[elevation]} ${className}`
    : elevationClass[elevation];
  const shared = {
    className: classes,
    onClick,
    "data-testid": testId,
    id,
    role,
    tabIndex,
    "aria-label": ariaLabel,
  };

  if (!interactive || reduceMotion) {
    return <div {...shared}>{children}</div>;
  }

  return (
    <motion.div
      {...shared}
      className={`${classes} rex-surface--interactive`}
      data-motion-tier="active"
      whileHover={{ boxShadow: "var(--rex-shadow-glow-working)" }}
      transition={pressSpring}
    >
      {children}
    </motion.div>
  );
}

export function Hairline({ className, ...rest }: HTMLAttributes<HTMLHRElement>) {
  const classes = className ? `rex-hairline ${className}` : "rex-hairline";
  return <hr className={classes} {...rest} />;
}

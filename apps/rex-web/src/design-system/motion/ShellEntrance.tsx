import { motion, useReducedMotion } from "framer-motion";
import type { ReactNode } from "react";

interface Props {
  children: ReactNode;
}

export function ShellEntrance({ children }: Props) {
  const reduceMotion = useReducedMotion();

  if (reduceMotion) {
    return (
      <div className="rex-shell-entrance" data-shell-revealed="yes">
        {children}
      </div>
    );
  }

  return (
    <motion.div
      className="rex-shell-entrance"
      data-motion-tier="ambient"
      data-shell-revealed="yes"
      initial={false}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.35, ease: [0.22, 1, 0.36, 1] }}
    >
      {children}
    </motion.div>
  );
}

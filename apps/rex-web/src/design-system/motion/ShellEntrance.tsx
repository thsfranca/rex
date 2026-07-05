import { motion, useReducedMotion } from "framer-motion";
import type { ReactNode } from "react";
import { useMotionOrchestrator } from "./orchestrator";

interface Props {
  children: ReactNode;
}

export function ShellEntrance({ children }: Props) {
  const reduceMotion = useReducedMotion();
  const { connectFade } = useMotionOrchestrator();
  const visible = connectFade > 0;

  if (reduceMotion) {
    return <>{children}</>;
  }

  return (
    <motion.div
      className="rex-shell-entrance"
      data-motion-tier="ambient"
      initial={{ opacity: 0, y: 12 }}
      animate={visible ? { opacity: 1, y: 0 } : { opacity: 0, y: 12 }}
      transition={{ duration: 0.4, ease: [0.22, 1, 0.36, 1] }}
    >
      {children}
    </motion.div>
  );
}

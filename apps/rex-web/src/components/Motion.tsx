import { motion, useReducedMotion } from "framer-motion";
import {
  activeTransition,
  ambientTransition,
  messageVariants,
  staggerChildren,
  statusPulseTransition,
  timelineItemVariants,
} from "../design-system/motion";

interface MessageProps {
  className: string;
  children: React.ReactNode;
  index?: number;
  "data-testid"?: string;
}

export function MotionMessage({
  className,
  children,
  index = 0,
  "data-testid": testId,
}: MessageProps) {
  const reduceMotion = useReducedMotion();

  if (reduceMotion) {
    return (
      <div className={className} data-testid={testId}>
        {children}
      </div>
    );
  }

  return (
    <motion.div
      className={className}
      data-testid={testId}
      data-motion-tier="active"
      initial="hidden"
      animate="visible"
      variants={messageVariants}
      transition={{ ...activeTransition, delay: index * staggerChildren }}
    >
      {children}
    </motion.div>
  );
}

export function MotionTimelineItem({
  children,
  index = 0,
}: {
  children: React.ReactNode;
  index?: number;
}) {
  const reduceMotion = useReducedMotion();

  if (reduceMotion) {
    return <li>{children}</li>;
  }

  return (
    <motion.li
      className="timeline-row-motion"
      data-motion-tier="ambient"
      initial="hidden"
      animate="visible"
      variants={timelineItemVariants}
      transition={{
        ...ambientTransition,
        type: "spring",
        stiffness: 260,
        damping: 22,
        delay: index * staggerChildren,
      }}
      whileHover={{ scale: 1.02, x: 4 }}
    >
      {children}
    </motion.li>
  );
}

export function MotionStatusDot({
  working,
  id,
  testId,
}: {
  working: boolean;
  id?: string;
  testId?: string;
}) {
  const reduceMotion = useReducedMotion();

  return (
    <motion.span
      className={`status-dot${working ? " working" : ""}`}
      id={id}
      data-testid={testId}
      data-motion-tier={working ? "ambient" : "idle"}
      animate={
        reduceMotion || !working
          ? { opacity: 1, scale: 1 }
          : { opacity: [1, 0.45, 1], scale: [1, 1.18, 1] }
      }
      transition={reduceMotion || !working ? { duration: 0 } : statusPulseTransition}
    />
  );
}

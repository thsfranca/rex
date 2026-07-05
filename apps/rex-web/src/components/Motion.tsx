import { motion, useReducedMotion } from "framer-motion";

const messageVariants = {
  hidden: { opacity: 0, y: 8 },
  visible: { opacity: 1, y: 0 },
};

interface Props {
  className: string;
  children: React.ReactNode;
  "data-testid"?: string;
}

export function MotionMessage({ className, children, "data-testid": testId }: Props) {
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
      transition={{ duration: 0.25, ease: [0.25, 1, 0.5, 1] }}
    >
      {children}
    </motion.div>
  );
}

export function MotionTimelineItem({ children }: { children: React.ReactNode }) {
  const reduceMotion = useReducedMotion();

  if (reduceMotion) {
    return <li>{children}</li>;
  }

  return (
    <motion.li
      data-motion-tier="ambient"
      initial={{ opacity: 0, x: 8 }}
      animate={{ opacity: 1, x: 0 }}
      transition={{ duration: 0.4, ease: [0.33, 1, 0.68, 1] }}
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
          ? { opacity: 1 }
          : { opacity: [1, 0.4, 1] }
      }
      transition={
        reduceMotion || !working
          ? { duration: 0 }
          : { duration: 1.2, repeat: Infinity, ease: "easeInOut" }
      }
    />
  );
}

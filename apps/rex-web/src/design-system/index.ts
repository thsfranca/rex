export {
  Button,
  IconButton,
  Badge,
  Text,
  Heading,
  Surface,
  Hairline,
  Stack,
  Grid,
  Modal,
  StatusDot,
  SegmentedControl,
  Textarea,
} from "./primitives";

export { ShellGrid } from "./layout";
export type { ShellGridProps } from "./layout";

export {
  MOTION_TIERS,
  motionTierAttr,
  ambientTransition,
  activeTransition,
  modalTransition,
  modalSpringTransition,
  messageVariants,
  timelineItemVariants,
  modalVariants,
  bannerVariants,
  statusPulseTransition,
  pressSpring,
  staggerChildren,
  motionOrchestrator,
  useMotionOrchestrator,
  useOrchestratorPhaseBinding,
  useOrchestratorErrorBinding,
  useOrchestratorStreamBinding,
  useSpringScalar,
  MotionPressable,
  MotionBanner,
  MotionSessionCard,
  ShellEntrance,
  AnimatedModal,
} from "./motion";
export type { MotionTier, AnimatedModalProps } from "./motion";

export { rexColorTokens, rexMotionTokens } from "./theme/electric-alive";
export type { RexColorToken, RexMotionToken } from "./theme/electric-alive";

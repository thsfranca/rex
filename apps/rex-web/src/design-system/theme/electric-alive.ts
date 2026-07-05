export const rexColorTokens = [
  "--rex-surface-base",
  "--rex-surface-raised",
  "--rex-surface-overlay",
  "--rex-surface-dimmed",
  "--rex-text-primary",
  "--rex-text-secondary",
  "--rex-hairline-default",
  "--rex-hairline-focus",
  "--rex-border-subtle",
  "--rex-status-success",
  "--rex-status-working",
  "--rex-status-error",
  "--rex-accent-glow",
  "--rex-glow-working",
  "--rex-glow-error",
  "--rex-hairline-flux",
  "--rex-shimmer-highlight",
  "--rex-particle-fill",
] as const;

export const rexMotionTokens = [
  "--rex-duration-ambient",
  "--rex-duration-active",
  "--rex-duration-modal",
  "--rex-ease-ambient",
  "--rex-ease-active",
  "--rex-spring-modal-stiffness",
  "--rex-spring-modal-damping",
] as const;

export type RexColorToken = (typeof rexColorTokens)[number];
export type RexMotionToken = (typeof rexMotionTokens)[number];

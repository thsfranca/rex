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
] as const;

export const rexMotionTokens = [
  "--rex-duration-ambient",
  "--rex-duration-active",
  "--rex-duration-modal",
  "--rex-ease-ambient",
  "--rex-ease-active",
] as const;

export type RexColorToken = (typeof rexColorTokens)[number];
export type RexMotionToken = (typeof rexMotionTokens)[number];

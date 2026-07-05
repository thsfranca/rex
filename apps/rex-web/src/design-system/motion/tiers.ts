export const MOTION_TIERS = ["idle", "ambient", "active", "cinematic"] as const;
export type MotionTier = (typeof MOTION_TIERS)[number];

export function motionTierAttr(tier: MotionTier): { "data-motion-tier": MotionTier } {
  return { "data-motion-tier": tier };
}

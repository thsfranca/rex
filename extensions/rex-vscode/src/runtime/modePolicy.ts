import type { InteractionMode, ModePolicy } from "../shared/messages";

const POLICY_BY_MODE: Record<InteractionMode, ModePolicy> = {
  ask: {
    mode: "ask",
    canMutateFiles: false,
    requiresExecutionApproval: false,
    requiresMutationApproval: true,
    summary: "Research and explain only. File mutations are blocked.",
  },
  plan: {
    mode: "plan",
    canMutateFiles: true,
    requiresExecutionApproval: false,
    requiresMutationApproval: true,
    summary: "Plan-first workflow. Mutating actions require approval.",
  },
  agent: {
    mode: "agent",
    canMutateFiles: true,
    requiresExecutionApproval: true,
    requiresMutationApproval: true,
    summary: "Guarded execution mode with approval checkpoints.",
  },
};

export function resolveModePolicy(mode: InteractionMode): ModePolicy {
  return POLICY_BY_MODE[mode];
}

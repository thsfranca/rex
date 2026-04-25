import { describe, expect, it } from "vitest";

import { resolveModePolicy } from "../runtime/modePolicy";

describe("resolveModePolicy", () => {
  it("blocks mutations in ask mode", () => {
    const policy = resolveModePolicy("ask");
    expect(policy.canMutateFiles).toBe(false);
    expect(policy.requiresExecutionApproval).toBe(false);
    expect(policy.requiresMutationApproval).toBe(true);
  });

  it("requires mutation approval in plan mode", () => {
    const policy = resolveModePolicy("plan");
    expect(policy.canMutateFiles).toBe(true);
    expect(policy.requiresExecutionApproval).toBe(false);
    expect(policy.requiresMutationApproval).toBe(true);
  });

  it("requires execution and mutation approvals in agent mode", () => {
    const policy = resolveModePolicy("agent");
    expect(policy.canMutateFiles).toBe(true);
    expect(policy.requiresExecutionApproval).toBe(true);
    expect(policy.requiresMutationApproval).toBe(true);
  });
});

import { describe, expect, it } from "vitest";

import { applyProductAgentOverlayForTest } from "../workspace/binding";

describe("applyProductAgentOverlay", () => {
  it("merges rex-agent sidecar and agent.approvals_enabled", () => {
    const result = applyProductAgentOverlayForTest({ version: 1 });
    const sidecars = result.sidecars as Record<string, unknown>;
    expect(sidecars.active).toBe("agent");
    expect(sidecars.required).toBe(true);
    const list = sidecars.list as Array<Record<string, unknown>>;
    expect(list.some((entry) => entry.name === "agent" && entry.binary === "rex-agent")).toBe(true);

    const agent = result.agent as Record<string, unknown>;
    expect(agent.approvals_enabled).toBe(true);
  });

  it("preserves existing sidecar list entries", () => {
    const result = applyProductAgentOverlayForTest({
      version: 1,
      sidecars: {
        list: [{ name: "custom", binary: "other", enabled: false }],
      },
    });
    const list = (result.sidecars as Record<string, unknown>).list as Array<Record<string, unknown>>;
    expect(list.some((entry) => entry.name === "custom")).toBe(true);
    expect(list.some((entry) => entry.name === "agent")).toBe(true);
  });
});

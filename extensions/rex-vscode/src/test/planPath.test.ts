import { describe, expect, it } from "vitest";

import { defaultPlanSavePath, normalizePlanSavePath, validatePlanSavePath } from "../runtime/planPath";

describe("planPath", () => {
  it("normalizes bare filenames under .rex/plans/", () => {
    expect(normalizePlanSavePath("feature.md")).toBe(".rex/plans/feature.md");
    expect(normalizePlanSavePath(".rex/plans/feature.md")).toBe(".rex/plans/feature.md");
  });

  it("accepts valid plan save paths", () => {
    expect(validatePlanSavePath("feature.md")).toEqual({
      ok: true,
      normalized: ".rex/plans/feature.md",
    });
  });

  it("rejects paths outside .rex/plans/", () => {
    const result = validatePlanSavePath("src/main.rs");
    expect(result.ok).toBe(false);
  });

  it("derives a default slug from the plan title", () => {
    expect(defaultPlanSavePath("Planning tools slice")).toBe(".rex/plans/planning-tools-slice.md");
  });
});

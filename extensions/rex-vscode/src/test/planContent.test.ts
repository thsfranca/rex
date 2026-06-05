import { describe, expect, it } from "vitest";

import { formatPlanDetailMarkdown, parseClarifyQuestions } from "../runtime/planContent";

describe("planContent", () => {
  it("formats ready plan JSON into markdown", () => {
    const detail = JSON.stringify({
      steps: [{ id: "1", summary: "Add hub", files: ["docs/PLANNING_TOOLS.md"] }],
      risks: ["Contract drift"],
      open_questions: [],
    });
    const markdown = formatPlanDetailMarkdown("Planning tools slice", detail);
    expect(markdown).toContain("# Planning tools slice");
    expect(markdown).toContain("Add hub");
    expect(markdown).toContain("Contract drift");
  });

  it("parses clarify questions from plan detail JSON", () => {
    const detail = JSON.stringify([
      { id: "q1", prompt: "Target surface?", options: ["Extension only", "Full stack"] },
    ]);
    expect(parseClarifyQuestions(detail)).toEqual([
      { id: "q1", prompt: "Target surface?", options: ["Extension only", "Full stack"] },
    ]);
  });
});

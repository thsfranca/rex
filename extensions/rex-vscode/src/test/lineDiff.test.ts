import { describe, expect, it } from "vitest";

import { computeLineDiff, diffStats } from "../../webview/diff/lineDiff";

describe("computeLineDiff", () => {
  it("marks added and removed lines", () => {
    const lines = computeLineDiff("alpha\nbeta", "alpha\ngamma");
    expect(lines.some((line) => line.kind === "remove" && line.text === "beta")).toBe(true);
    expect(lines.some((line) => line.kind === "add" && line.text === "gamma")).toBe(true);
    expect(diffStats(lines)).toEqual({ added: 1, removed: 1 });
  });

  it("preserves unchanged context lines", () => {
    const lines = computeLineDiff("keep\nold\nstay", "keep\nnew\nstay");
    expect(lines.filter((line) => line.kind === "context").map((line) => line.text)).toEqual([
      "keep",
      "stay",
    ]);
  });
});

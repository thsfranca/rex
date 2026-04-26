import { describe, expect, it } from "vitest";

import { appendWithByteCap, elideForTooltip } from "../runtime/cappedString";

describe("elideForTooltip", () => {
  it("returns short strings unchanged", () => {
    expect(elideForTooltip("ok", 10)).toBe("ok");
  });

  it("truncates with an ellipsis", () => {
    expect(elideForTooltip("0123456789", 4)).toBe("012…");
  });
});

describe("appendWithByteCap", () => {
  it("keeps the string when under the cap", () => {
    expect(appendWithByteCap("a", "b", 8)).toBe("ab");
  });

  it("retains head and tail with a marker when over the cap", () => {
    const huge = "x".repeat(100_000);
    const out = appendWithByteCap("", huge, 20_000);
    expect(out.length).toBeLessThanOrEqual(20_000);
    expect(out).toContain("[rex: stderr truncated]");
    expect(out.startsWith("xx")).toBe(true);
  });
});

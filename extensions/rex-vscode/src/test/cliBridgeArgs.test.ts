import { describe, expect, it } from "vitest";

import { buildCompleteNdjsonArgs } from "../runtime/cliBridge";

describe("buildCompleteNdjsonArgs", () => {
  it("omits --model when stream model is empty", () => {
    expect(buildCompleteNdjsonArgs("hi", { mode: "ask" })).toEqual([
      "complete",
      "hi",
      "--format",
      "ndjson",
      "--mode",
      "ask",
    ]);
  });

  it("includes --model when stream model is set", () => {
    expect(buildCompleteNdjsonArgs("hi", { mode: "agent", model: "llama3.2" })).toEqual([
      "complete",
      "hi",
      "--format",
      "ndjson",
      "--mode",
      "agent",
      "--model",
      "llama3.2",
    ]);
  });
});

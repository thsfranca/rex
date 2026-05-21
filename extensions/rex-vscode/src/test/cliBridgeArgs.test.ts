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

  it("includes --approval-id when approvalId is set", () => {
    expect(
      buildCompleteNdjsonArgs("hi", { mode: "agent", approvalId: "apr-msg-1" }),
    ).toEqual([
      "complete",
      "hi",
      "--format",
      "ndjson",
      "--mode",
      "agent",
      "--approval-id",
      "apr-msg-1",
    ]);
  });

  it("omits --approval-id when approvalId is empty", () => {
    expect(buildCompleteNdjsonArgs("hi", { mode: "agent", approvalId: "" })).toEqual([
      "complete",
      "hi",
      "--format",
      "ndjson",
      "--mode",
      "agent",
    ]);
  });
});

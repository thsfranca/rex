import { describe, expect, it } from "vitest";

import { streamFailureWantsSetupHint } from "../runtime/userActionableFailure";

describe("streamFailureWantsSetupHint", () => {
  it("is true for daemon, spawn, and timeout codes", () => {
    expect(streamFailureWantsSetupHint("daemon_unavailable")).toBe(true);
    expect(streamFailureWantsSetupHint("spawn_failed")).toBe(true);
    expect(streamFailureWantsSetupHint("stream_timeout")).toBe(true);
  });

  it("is false for cancel and parse-style errors", () => {
    expect(streamFailureWantsSetupHint("cancelled")).toBe(false);
    expect(streamFailureWantsSetupHint("invalid_response")).toBe(false);
    expect(streamFailureWantsSetupHint("unknown")).toBe(false);
  });
});

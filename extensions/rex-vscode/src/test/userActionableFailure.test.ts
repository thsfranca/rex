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

  it("is true when message mentions sidecar or approval setup", () => {
    expect(
      streamFailureWantsSetupHint(
        "unknown",
        "sidecar required but REX_SIDECAR_ENABLED is off",
      ),
    ).toBe(true);
    expect(
      streamFailureWantsSetupHint(
        "invalid_response",
        "agent execution denied by approval gate",
      ),
    ).toBe(true);
    expect(streamFailureWantsSetupHint("unknown", "unrelated parse failure")).toBe(false);
  });
});

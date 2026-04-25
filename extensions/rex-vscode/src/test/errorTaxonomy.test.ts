import { describe, expect, it } from "vitest";

import { classifyStreamError, classifyStreamErrorMessage } from "../runtime/errorTaxonomy";

describe("errorTaxonomy", () => {
  it("respects explicit upstream error code", () => {
    const classified = classifyStreamError({
      kind: "error",
      message: "daemon is unavailable at /tmp/rex.sock",
      code: "daemon_unavailable",
    });
    expect(classified).toEqual({
      code: "daemon_unavailable",
      message: "daemon is unavailable at /tmp/rex.sock",
      retryable: true,
    });
  });

  it("maps cancellation message deterministically", () => {
    const classified = classifyStreamErrorMessage("cancelled");
    expect(classified).toEqual({
      code: "cancelled",
      message: "cancelled",
      retryable: true,
    });
  });

  it("maps malformed NDJSON to invalid_response", () => {
    const classified = classifyStreamErrorMessage("Malformed NDJSON line: {oops}");
    expect(classified.code).toBe("invalid_response");
    expect(classified.retryable).toBe(false);
  });

  it("falls back to unknown for unmatched messages", () => {
    const classified = classifyStreamErrorMessage("mystery problem");
    expect(classified.code).toBe("unknown");
    expect(classified.retryable).toBe(false);
  });
});

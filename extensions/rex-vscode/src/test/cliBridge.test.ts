import { describe, expect, it } from "vitest";

import { parseStatusOutput } from "../runtime/cliBridge";

describe("parseStatusOutput", () => {
  it("parses the expected three-line status response", () => {
    const snapshot = parseStatusOutput(
      [
        "daemon_version: 0.1.0",
        "uptime_seconds: 42",
        "active_model_id: mock",
        "",
      ].join("\n"),
    );
    expect(snapshot.daemonVersion).toBe("0.1.0");
    expect(snapshot.uptimeSeconds).toBe(42);
    expect(snapshot.activeModelId).toBe("mock");
    expect(snapshot.capturedAt).toBeTypeOf("number");
  });

  it("tolerates trailing whitespace and CRLF endings", () => {
    const snapshot = parseStatusOutput(
      "daemon_version: 0.2.0 \r\nuptime_seconds: 1 \r\nactive_model_id: \r\n",
    );
    expect(snapshot.daemonVersion).toBe("0.2.0");
    expect(snapshot.uptimeSeconds).toBe(1);
    expect(snapshot.activeModelId).toBe("");
  });

  it("throws when required fields are missing", () => {
    expect(() =>
      parseStatusOutput("daemon_version: 0.1.0\nuptime_seconds: 1\n"),
    ).toThrow(/missing required fields/);
  });

  it("throws when uptime is not numeric", () => {
    expect(() =>
      parseStatusOutput(
        "daemon_version: 0.1.0\nuptime_seconds: forever\nactive_model_id: x\n",
      ),
    ).toThrow(/non-numeric uptime_seconds/);
  });
});

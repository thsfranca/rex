import { describe, expect, it } from "vitest";

import {
  appendCliExecutableNotFoundHint,
  appendDaemonExecutableNotFoundHint,
  appendStreamSetupHint,
  EXTENSION_LOCAL_E2E_DOC_PATH,
  isExecutableNotFoundError,
} from "../runtime/spawnExecutableHints";

describe("spawnExecutableHints", () => {
  it("detects ENOENT-shaped errors", () => {
    expect(isExecutableNotFoundError(Object.assign(new Error("spawn ENOENT"), { code: "ENOENT" }))).toBe(
      true,
    );
    expect(isExecutableNotFoundError(new Error("other"))).toBe(false);
    expect(isExecutableNotFoundError(null)).toBe(false);
  });

  it("appends cli hint only for executable-not-found", () => {
    const enoent = Object.assign(new Error("spawn x ENOENT"), { code: "ENOENT" as const });
    expect(appendCliExecutableNotFoundHint(enoent, "spawn x ENOENT")).toContain(
      EXTENSION_LOCAL_E2E_DOC_PATH,
    );
    expect(appendCliExecutableNotFoundHint(enoent, "spawn x ENOENT")).toContain("rex.cliPath");
    expect(appendCliExecutableNotFoundHint(new Error("other"), "other")).toBe("other");
  });

  it("appends daemon hint only for executable-not-found", () => {
    const enoent = Object.assign(new Error("spawn y ENOENT"), { code: "ENOENT" as const });
    expect(appendDaemonExecutableNotFoundHint(enoent, "spawn y ENOENT")).toContain(
      EXTENSION_LOCAL_E2E_DOC_PATH,
    );
    expect(appendDaemonExecutableNotFoundHint(enoent, "spawn y ENOENT")).toContain(
      "rex.daemonBinaryPath",
    );
    expect(appendDaemonExecutableNotFoundHint(new Error("other"), "other")).toBe("other");
  });

  it("appends stream setup hints for sidecar and HTTP failures", () => {
    expect(
      appendStreamSetupHint("sidecar required but unavailable: startup timeout"),
    ).toContain("REX_SIDECAR_ENABLED");
    expect(
      appendStreamSetupHint("inference runtime configuration: missing base url"),
    ).toContain("REX_OPENAI_COMPAT");
    expect(appendStreamSetupHint("unrelated failure")).toBe("unrelated failure");
  });
});

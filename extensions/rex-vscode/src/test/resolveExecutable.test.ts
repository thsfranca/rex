import * as os from "node:os";
import * as path from "node:path";
import { describe, expect, it } from "vitest";

import { resolveRexExecutable } from "../config/resolveExecutable";

describe("resolveRexExecutable", () => {
  it("returns configured path when not the default rex name", () => {
    expect(resolveRexExecutable("/opt/rex/bin/rex")).toBe("/opt/rex/bin/rex");
  });

  it("returns default rex when no well-known binary exists", () => {
    expect(resolveRexExecutable("rex", () => false)).toBe("rex");
  });

  it("resolves ~/.cargo/bin/rex when default rex and file exists", () => {
    const cargoRex = path.join(os.homedir(), ".cargo", "bin", "rex");
    expect(
      resolveRexExecutable("rex", (candidate) => candidate === cargoRex),
    ).toBe(cargoRex);
  });

  it("preserves explicit rex override even when cargo bin exists", () => {
    const cargoRex = path.join(os.homedir(), ".cargo", "bin", "rex");
    expect(resolveRexExecutable("/usr/local/bin/rex", () => true)).toBe(
      "/usr/local/bin/rex",
    );
    expect(resolveRexExecutable("rex", () => true)).toBe(cargoRex);
  });
});

import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { afterEach, describe, expect, it } from "vitest";

import { ensureProjectRexConfig } from "../workspace/binding";

describe("ensureProjectRexConfig", () => {
  const dirs: string[] = [];

  afterEach(() => {
    for (const dir of dirs.splice(0)) {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });

  it("writes workspace.root as absolute path", () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "rex-bind-"));
    dirs.push(root);
    ensureProjectRexConfig(root);
    const configPath = path.join(root, ".rex", "config.json");
    const parsed = JSON.parse(fs.readFileSync(configPath, "utf8")) as {
      workspace?: { root?: string };
      daemon?: { socket_scope?: string };
    };
    expect(parsed.workspace?.root).toBe(root);
    expect(parsed.daemon?.socket_scope).toBe("per_workspace");
  });

  it("preserves existing keys when merging workspace root", () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "rex-bind-"));
    dirs.push(root);
    fs.mkdirSync(path.join(root, ".rex"), { recursive: true });
    fs.writeFileSync(
      path.join(root, ".rex", "config.json"),
      JSON.stringify({ version: 1, inference: { runtime: "mock" } }),
    );
    ensureProjectRexConfig(root);
    const parsed = JSON.parse(
      fs.readFileSync(path.join(root, ".rex", "config.json"), "utf8"),
    ) as { inference?: { runtime?: string }; workspace?: { root?: string } };
    expect(parsed.inference?.runtime).toBe("mock");
    expect(parsed.workspace?.root).toBe(root);
  });
});

import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

import { NdjsonLineParser } from "../runtime/ndjsonParser";

// `npm test` and CI `run_extension_verify.sh` use cwd `extensions/rex-vscode`.
const happyPathFixture = path.resolve(
  process.cwd(),
  "../../fixtures/ndjson_contract/happy_path.ndjson",
);

describe("NDJSON cross-boundary fixture", () => {
  it("parses the shared repo fixture used by rex-cli conformance tests", () => {
    const raw = readFileSync(happyPathFixture, "utf8");
    const parser = new NdjsonLineParser();
    const events = parser.push(raw);
    expect(events).toEqual([
      { kind: "chunk", index: 0, text: "hello " },
      { kind: "chunk", index: 1, text: "world" },
      { kind: "done", index: 2 },
    ]);
  });
});

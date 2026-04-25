import { describe, expect, it } from "vitest";

import { splitByCodeBlocks } from "../../webview/streaming/codeBlocks";

describe("splitByCodeBlocks", () => {
  it("returns an empty array for empty input", () => {
    expect(splitByCodeBlocks("")).toEqual([]);
  });

  it("treats plain text as a single markdown segment", () => {
    const out = splitByCodeBlocks("hello world\nmore text\n");
    expect(out).toHaveLength(1);
    expect(out[0]).toEqual({ kind: "markdown", content: "hello world\nmore text\n" });
  });

  it("extracts a fenced code block with language", () => {
    const input = "intro\n```ts\nconst a = 1;\n```\nouter\n";
    const out = splitByCodeBlocks(input);
    expect(out).toEqual([
      { kind: "markdown", content: "intro\n" },
      { kind: "code", content: "const a = 1;\n", language: "ts" },
      { kind: "markdown", content: "\nouter\n" },
    ]);
  });

  it("extracts a fence without language", () => {
    const input = "```\nraw\n```";
    const out = splitByCodeBlocks(input);
    expect(out).toEqual([{ kind: "code", content: "raw\n", language: "" }]);
  });

  it("skips the markdown segment when whitespace-only", () => {
    const input = "\n\n```rust\nfn main() {}\n```\n";
    const out = splitByCodeBlocks(input);
    expect(out.map((s) => s.kind)).toEqual(["code"]);
  });
});

import { describe, expect, it } from "vitest";

import {
  MarkdownStream,
  findLastStableBoundary,
} from "../../webview/streaming/markdownStream";

describe("findLastStableBoundary", () => {
  it("returns 0 for empty input", () => {
    expect(findLastStableBoundary("")).toBe(0);
  });

  it("advances only at newline boundaries when no fence is open", () => {
    expect(findLastStableBoundary("hello")).toBe(0);
    expect(findLastStableBoundary("line 1\n")).toBe("line 1\n".length);
    expect(findLastStableBoundary("line 1\nhalf")).toBe("line 1\n".length);
  });

  it("keeps the boundary below an unterminated fence", () => {
    const input = "intro\n```ts\nconst x = 1;";
    expect(findLastStableBoundary(input)).toBe("intro\n".length);
  });

  it("advances past a closed fence", () => {
    const input = "intro\n```ts\nconst x = 1;\n```\n";
    expect(findLastStableBoundary(input)).toBe(input.length);
  });
});

describe("MarkdownStream", () => {
  it("streams trailing raw text until the first newline", () => {
    const stream = new MarkdownStream();
    const r = stream.push("hello");
    expect(r.html).toBe("");
    expect(r.trailingRaw).toBe("hello");
  });

  it("renders stable prefix when a newline arrives", () => {
    const stream = new MarkdownStream();
    stream.push("paragraph\n");
    const r = stream.push("continued...");
    expect(r.html).toContain("<p>paragraph</p>");
    expect(r.trailingRaw).toBe("continued...");
  });

  it("defers rendering while a fenced block is open", () => {
    const stream = new MarkdownStream();
    stream.push("intro\n```ts\n");
    const r = stream.push("const a = 1;");
    expect(r.html).toContain("<p>intro</p>");
    expect(r.trailingRaw).toContain("```ts");
  });

  it("renders the code block once the fence closes", () => {
    const stream = new MarkdownStream();
    stream.push("intro\n```ts\n");
    stream.push("const a = 1;\n");
    const r = stream.push("```\n");
    expect(r.html).toContain("<pre>");
    expect(r.trailingRaw).toBe("");
  });

  it("finalize flushes whatever is buffered", () => {
    const stream = new MarkdownStream();
    stream.push("only one line without newline");
    const r = stream.finalize();
    expect(r.trailingRaw).toBe("");
    expect(r.html).toContain("only one line");
  });
});

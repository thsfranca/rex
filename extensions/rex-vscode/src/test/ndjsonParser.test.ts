import { describe, expect, it } from "vitest";

import { NdjsonLineParser } from "../runtime/ndjsonParser";

describe("NdjsonLineParser", () => {
  it("parses a happy-path chunk-done stream line by line", () => {
    const parser = new NdjsonLineParser();
    const events = parser.push(
      '{"event":"chunk","index":0,"text":"hello "}\n' +
        '{"event":"chunk","index":1,"text":"world"}\n' +
        '{"event":"done","index":2}\n',
    );
    expect(events).toEqual([
      { kind: "chunk", index: 0, text: "hello " },
      { kind: "chunk", index: 1, text: "world" },
      { kind: "done", index: 2 },
    ]);
  });

  it("buffers partial lines across multiple pushes", () => {
    const parser = new NdjsonLineParser();
    const first = parser.push('{"event":"chunk","index":0,"te');
    expect(first).toEqual([]);
    const second = parser.push('xt":"hi"}\n');
    expect(second).toEqual([{ kind: "chunk", index: 0, text: "hi" }]);
  });

  it("flushes a final line without a trailing newline", () => {
    const parser = new NdjsonLineParser();
    parser.push('{"event":"done","index":5}');
    expect(parser.flush()).toEqual([{ kind: "done", index: 5 }]);
  });

  it("reports malformed lines as error events", () => {
    const parser = new NdjsonLineParser();
    const events = parser.push("not-json\n");
    expect(events).toHaveLength(1);
    expect(events[0]?.kind).toBe("error");
  });

  it("treats missing index on chunk as an error event", () => {
    const parser = new NdjsonLineParser();
    const events = parser.push('{"event":"chunk","text":"oops"}\n');
    expect(events).toEqual([
      { kind: "error", message: "chunk event missing numeric index" },
    ]);
  });

  it("surfaces error events from the upstream CLI", () => {
    const parser = new NdjsonLineParser();
    const events = parser.push('{"event":"error","message":"daemon unavailable"}\n');
    expect(events).toEqual([
      { kind: "error", message: "daemon unavailable" },
    ]);
  });

  it("rejects unknown event types without crashing", () => {
    const parser = new NdjsonLineParser();
    const events = parser.push('{"event":"weird"}\n');
    expect(events).toHaveLength(1);
    expect(events[0]?.kind).toBe("error");
  });

  it("ignores blank lines", () => {
    const parser = new NdjsonLineParser();
    const events = parser.push('\n\n{"event":"done","index":0}\n\n');
    expect(events).toEqual([{ kind: "done", index: 0 }]);
  });

  it("handles CRLF line endings", () => {
    const parser = new NdjsonLineParser();
    const events = parser.push('{"event":"done","index":1}\r\n');
    expect(events).toEqual([{ kind: "done", index: 1 }]);
  });
});

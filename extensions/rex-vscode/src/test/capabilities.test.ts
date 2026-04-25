import { describe, expect, it } from "vitest";

import { detectCapabilities } from "../platform/capabilities";

describe("detectCapabilities", () => {
  it("returns all-false flags in plain VS Code", () => {
    const caps = detectCapabilities({});
    expect(caps).toEqual({
      hasCursor: false,
      hasCursorPlugins: false,
      hasCursorMcp: false,
    });
  });

  it("returns all-false flags when cursor is a non-object value", () => {
    const caps = detectCapabilities({ cursor: 123 as unknown });
    expect(caps).toEqual({
      hasCursor: false,
      hasCursorPlugins: false,
      hasCursorMcp: false,
    });
  });

  it("detects Cursor with plugin support", () => {
    const caps = detectCapabilities({
      cursor: {
        plugins: { registerPath: () => undefined },
      },
    });
    expect(caps).toEqual({
      hasCursor: true,
      hasCursorPlugins: true,
      hasCursorMcp: false,
    });
  });

  it("detects Cursor with plugins and MCP support", () => {
    const caps = detectCapabilities({
      cursor: {
        plugins: { registerPath: () => undefined },
        mcp: { registerServer: () => undefined },
      },
    });
    expect(caps).toEqual({
      hasCursor: true,
      hasCursorPlugins: true,
      hasCursorMcp: true,
    });
  });

  it("ignores cursor sub-namespaces that are missing expected functions", () => {
    const caps = detectCapabilities({
      cursor: {
        plugins: {},
        mcp: { registerServer: "not-a-function" },
      },
    });
    expect(caps).toEqual({
      hasCursor: true,
      hasCursorPlugins: false,
      hasCursorMcp: false,
    });
  });
});

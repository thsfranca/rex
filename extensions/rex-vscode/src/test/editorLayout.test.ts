import { describe, expect, it } from "vitest";

import type { ChatLocationSetting } from "../config/settings";
import { resolveChatSidebar } from "../platform/editorLayout";

describe("resolveChatSidebar", () => {
  it("uses right secondary sidebar for auto when supported", () => {
    expect(resolveChatSidebar("auto", true)).toBe("right");
  });

  it("falls back to left activity bar for auto when secondary sidebar is unavailable", () => {
    expect(resolveChatSidebar("auto", false)).toBe("left");
  });

  it("honours explicit left placement", () => {
    expect(resolveChatSidebar("left", true)).toBe("left");
    expect(resolveChatSidebar("left", false)).toBe("left");
  });

  it("honours explicit right placement with fallback", () => {
    expect(resolveChatSidebar("right", true)).toBe("right");
    expect(resolveChatSidebar("right", false)).toBe("left");
  });

  it("hides sidebar containers for editor placement", () => {
    expect(resolveChatSidebar("editor", true)).toBe("none");
    expect(resolveChatSidebar("editor", false)).toBe("none");
  });

  it("covers every configured location", () => {
    const locations: ChatLocationSetting[] = ["auto", "right", "left", "editor"];
    for (const location of locations) {
      expect(["left", "right", "none"]).toContain(resolveChatSidebar(location, true));
    }
  });
});

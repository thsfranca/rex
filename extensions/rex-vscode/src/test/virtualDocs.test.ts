import { describe, expect, it, vi } from "vitest";

import { RexProposalProvider, REX_PROPOSAL_SCHEME } from "../editor/virtualDocs";

describe("RexProposalProvider", () => {
  it("stores and serves proposed content per id", () => {
    const provider = new RexProposalProvider();
    const uri = provider.register("stream-1", "initial content");
    expect(uri.scheme).toBe(REX_PROPOSAL_SCHEME);
    expect(uri.path).toBe("/stream-1");
    expect(provider.provideTextDocumentContent(uri)).toBe("initial content");
  });

  it("emits change events on update and returns new content", () => {
    const provider = new RexProposalProvider();
    const uri = provider.register("stream-2", "v1");
    const listener = vi.fn();
    provider.onDidChange(listener);
    provider.update("stream-2", "v2");
    expect(listener).toHaveBeenCalledTimes(1);
    expect(listener).toHaveBeenCalledWith(expect.objectContaining({ path: "/stream-2" }));
    expect(provider.provideTextDocumentContent(uri)).toBe("v2");
  });

  it("returns empty content after delete", () => {
    const provider = new RexProposalProvider();
    const uri = provider.register("stream-3", "something");
    provider.delete("stream-3");
    expect(provider.provideTextDocumentContent(uri)).toBe("");
  });
});

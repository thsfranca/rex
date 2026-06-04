import { describe, expect, it } from "vitest";

import { initialState, reducer } from "../../webview/appState";

function userSend(id: string, text: string) {
  return reducer(initialState, { type: "userSend", id, text, attachContext: false });
}

describe("appState reducer — cancel to idle", () => {
  it("clears streaming state after host streamError with cancelled code", () => {
    const streaming = reducer(userSend("turn-1", "hello"), {
      type: "hostMessage",
      payload: {
        type: "streamError",
        id: "turn-1",
        message: "cancelled",
        code: "cancelled",
        retryable: true,
      },
    });

    expect(streaming.streaming).toBe(false);
    expect(streaming.activeStreamId).toBeUndefined();
    const assistant = streaming.messages.find((msg) => msg.id === "turn-1");
    expect(assistant?.streaming).toBe(false);
    expect(assistant?.errorMessage).toBe("cancelled");
  });

  it("remains idle after a second terminal streamError for the same turn", () => {
    let state = userSend("turn-1", "hello");
    state = reducer(state, {
      type: "hostMessage",
      payload: {
        type: "streamError",
        id: "turn-1",
        message: "cancelled",
        code: "cancelled",
        retryable: true,
      },
    });
    state = reducer(state, {
      type: "hostMessage",
      payload: {
        type: "streamError",
        id: "turn-1",
        message: "cancelled",
        code: "cancelled",
        retryable: true,
      },
    });

    expect(state.streaming).toBe(false);
    expect(state.activeStreamId).toBeUndefined();
  });

  it("simulates 10+ turns ending with cancel streamError and returns idle", () => {
    let state = initialState;
    for (let turn = 0; turn < 10; turn += 1) {
      const id = `turn-${turn}`;
      state = reducer(state, { type: "userSend", id, text: `prompt ${turn}`, attachContext: false });
      state = reducer(state, {
        type: "hostMessage",
        payload: { type: "streamDone", id },
      });
    }

    const finalId = "turn-final";
    state = reducer(state, { type: "userSend", id: finalId, text: "last", attachContext: false });
    state = reducer(state, {
      type: "hostMessage",
      payload: {
        type: "streamError",
        id: finalId,
        message: "cancelled",
        code: "cancelled",
        retryable: true,
      },
    });

    expect(state.streaming).toBe(false);
    expect(state.activeStreamId).toBeUndefined();
    expect(state.messages.filter((msg) => msg.streaming).length).toBe(0);
    expect(state.messages.length).toBe(22);
  });

  it("does not clear active streaming when streamDone targets a stale id", () => {
    let state = userSend("active", "current");
    state = reducer(state, { type: "userSend", id: "active-2", text: "next", attachContext: false });
    expect(state.activeStreamId).toBe("active-2");
    expect(state.streaming).toBe(true);

    state = reducer(state, {
      type: "hostMessage",
      payload: { type: "streamDone", id: "active" },
    });

    expect(state.streaming).toBe(true);
    expect(state.activeStreamId).toBe("active-2");
    const staleAssistant = state.messages.find((msg) => msg.id === "active");
    expect(staleAssistant?.streaming).toBe(false);
  });

  it("does not clear active streaming when streamError targets a stale id", () => {
    let state = userSend("active", "current");
    state = reducer(state, { type: "userSend", id: "active-2", text: "next", attachContext: false });

    state = reducer(state, {
      type: "hostMessage",
      payload: {
        type: "streamError",
        id: "active",
        message: "cancelled",
        code: "cancelled",
        retryable: true,
      },
    });

    expect(state.streaming).toBe(true);
    expect(state.activeStreamId).toBe("active-2");
  });

  it("caps timeline entries at 20 under repeated executionStep messages", () => {
    let state = initialState;
    for (let index = 0; index < 25; index += 1) {
      state = reducer(state, {
        type: "hostMessage",
        payload: {
          type: "executionStep",
          payload: { id: `step-${index}`, phase: "running", summary: `step ${index}` },
        },
      });
    }

    expect(state.timeline).toHaveLength(20);
    expect(state.timeline[0]?.id).toBe("step-5");
    expect(state.timeline[19]?.id).toBe("step-24");
  });
});

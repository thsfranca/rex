import * as path from "node:path";
import { describe, expect, it } from "vitest";

import { streamComplete } from "../runtime/streamClient";
import type { StreamEvent } from "../runtime/ndjsonParser";

function fixtureBinary(name: string): string {
  return path.resolve(__dirname, "fixtures", name);
}

async function collect(iter: AsyncIterable<StreamEvent>): Promise<StreamEvent[]> {
  const events: StreamEvent[] = [];
  for await (const event of iter) {
    events.push(event);
  }
  return events;
}

describe("streamComplete", () => {
  it("yields chunk events and terminates with done on a normal run", async () => {
    const events = await collect(
      streamComplete({ cliPath: fixtureBinary("cli_success.sh") }, {
        prompt: "hi",
      }),
    );
    expect(events).toEqual([
      { kind: "chunk", index: 0, text: "hello " },
      { kind: "chunk", index: 1, text: "world" },
      { kind: "done", index: 2 },
    ]);
  });

  it("propagates error events from the CLI", async () => {
    const events = await collect(
      streamComplete({ cliPath: fixtureBinary("cli_error.sh") }, {
        prompt: "hi",
      }),
    );
    expect(events).toEqual([
      { kind: "error", message: "daemon unavailable" },
    ]);
  });

  it("emits a synthetic error when the CLI exits without a terminal event", async () => {
    const events = await collect(
      streamComplete({ cliPath: fixtureBinary("cli_silent_exit.sh") }, {
        prompt: "hi",
      }),
    );
    expect(events.length).toBeGreaterThanOrEqual(1);
    expect(events.at(-1)?.kind).toBe("error");
  });

  it("cancels an in-flight stream via AbortSignal", async () => {
    const controller = new AbortController();
    const iter = streamComplete({ cliPath: fixtureBinary("cli_slow.sh") }, {
      prompt: "hi",
      signal: controller.signal,
    });
    const reader = iter[Symbol.asyncIterator]();
    setTimeout(() => controller.abort(), 50);
    const events: StreamEvent[] = [];
    let isFinished = false;
    while (!isFinished) {
      const { value, done } = await reader.next();
      if (done) {
        isFinished = true;
        break;
      }
      events.push(value);
    }
    expect(events.at(-1)?.kind).toBe("error");
    expect(events.some((event) => event.kind === "error" && event.message === "cancelled")).toBe(
      true,
    );
  });
});

import * as path from "node:path";
import { describe, expect, it } from "vitest";

import { streamComplete } from "../runtime/streamClient";
import type { StreamEvent } from "../runtime/ndjsonParser";
import { EXTENSION_LOCAL_E2E_DOC_PATH } from "../runtime/spawnExecutableHints";

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

function terminalEvents(events: StreamEvent[]): StreamEvent[] {
  return events.filter((event) => event.kind === "done" || event.kind === "error");
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
    expect(terminalEvents(events)).toHaveLength(1);
  });

  it("propagates error events from the CLI", async () => {
    const events = await collect(
      streamComplete({ cliPath: fixtureBinary("cli_error.sh") }, {
        prompt: "hi",
      }),
    );
    expect(events).toEqual([
      { kind: "error", message: "daemon unavailable", code: "daemon_unavailable" },
    ]);
    expect(terminalEvents(events)).toHaveLength(1);
  });

  it("ignores duplicate terminal markers after done", async () => {
    const events = await collect(
      streamComplete({ cliPath: fixtureBinary("cli_multi_terminal.sh") }, {
        prompt: "hi",
      }),
    );
    expect(events).toEqual([
      { kind: "chunk", index: 0, text: "hello" },
      { kind: "done", index: 1 },
    ]);
    expect(terminalEvents(events)).toHaveLength(1);
  });

  it("emits a synthetic error when the CLI exits without a terminal event", async () => {
    const events = await collect(
      streamComplete({ cliPath: fixtureBinary("cli_silent_exit.sh") }, {
        prompt: "hi",
      }),
    );
    expect(events.length).toBeGreaterThanOrEqual(1);
    expect(events.at(-1)?.kind).toBe("error");
    expect(terminalEvents(events)).toHaveLength(1);
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
    expect(events.some((event) => event.kind === "error" && event.code === "cancelled")).toBe(true);
    expect(terminalEvents(events)).toHaveLength(1);
  });

  it("includes onboarding hint when rex-cli executable is missing", async () => {
    const events = await collect(
      streamComplete({ cliPath: "/__rex_vitest_nonexistent__/rex-cli" }, {
        prompt: "hi",
      }),
    );
    const last = events.at(-1);
    expect(last?.kind).toBe("error");
    if (last?.kind === "error") {
      expect(last.message).toMatch(/failed to spawn rex-cli/);
      expect(last.message).toContain(EXTENSION_LOCAL_E2E_DOC_PATH);
      expect(last.message).toContain("rex.cliPath");
    }
  });

  it("emits a single cancellation error when the signal is already aborted", async () => {
    const controller = new AbortController();
    controller.abort();
    const events = await collect(
      streamComplete({ cliPath: fixtureBinary("cli_slow.sh") }, {
        prompt: "hi",
        signal: controller.signal,
      }),
    );
    expect(events).toEqual([{ kind: "error", message: "cancelled", code: "cancelled" }]);
    expect(terminalEvents(events)).toHaveLength(1);
  });

  it("classifies malformed NDJSON as invalid_response", async () => {
    const events = await collect(
      streamComplete({ cliPath: fixtureBinary("cli_invalid_ndjson.sh") }, {
        prompt: "hi",
      }),
    );
    const last = events.at(-1);
    expect(last?.kind).toBe("error");
    if (last?.kind === "error") {
      expect(last.code).toBe("invalid_response");
    }
    expect(terminalEvents(events)).toHaveLength(1);
  });

  it("emits trace lifecycle callbacks with terminal metrics", async () => {
    const traces: string[] = [];
    const events = await collect(
      streamComplete({ cliPath: fixtureBinary("cli_success.sh") }, {
        prompt: "hi",
        onLifecycle: (event) => {
          if (event.phase === "start") {
            traces.push(`start:${event.traceId}`);
          } else {
            traces.push(`terminal:${event.traceId}:${event.terminalCode}:${event.elapsedMs ?? -1}`);
          }
        },
      }),
    );
    expect(events.at(-1)).toEqual({ kind: "done", index: 2 });
    expect(traces).toHaveLength(2);
    expect(traces[0]?.startsWith("start:rex-")).toBe(true);
    expect(traces[1]).toContain(":done:");
  });
});

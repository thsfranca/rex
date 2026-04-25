import { spawnCompleteStream, type CliBridgeOptions } from "./cliBridge";
import { NdjsonLineParser, type StreamEvent } from "./ndjsonParser";

export interface StreamRequest {
  readonly prompt: string;
  readonly signal?: AbortSignal;
}

/**
 * Async-iterable stream of typed events for a single `rex-cli complete` call.
 *
 * Rules:
 * - The stream terminates exactly once (either `done` or `error`).
 * - If the consumer aborts via `AbortSignal`, the child process is killed and
 *   a terminal `error` event with message `cancelled` is emitted.
 * - If stdout closes without a terminal event, a synthetic `error` event is
 *   emitted so consumers always see a terminal marker.
 */
export async function* streamComplete(
  options: CliBridgeOptions,
  request: StreamRequest,
): AsyncIterable<StreamEvent> {
  const { child, dispose } = spawnCompleteStream(options, request.prompt);
  const parser = new NdjsonLineParser();
  const queue: StreamEvent[] = [];
  let pendingResolve: (() => void) | undefined;
  let terminalEvent: Extract<StreamEvent, { kind: "done" | "error" }> | undefined;
  let cleanupDone = false;

  const wakeConsumer = () => {
    pendingResolve?.();
    pendingResolve = undefined;
  };

  const pushEvent = (event: StreamEvent) => {
    if (terminalEvent !== undefined) {
      return;
    }
    queue.push(event);
    if (event.kind === "done" || event.kind === "error") {
      terminalEvent = event;
    }
    wakeConsumer();
  };

  const cleanup = () => {
    if (cleanupDone) {
      return;
    }
    cleanupDone = true;
    request.signal?.removeEventListener("abort", signalListener);
    if (!child.killed) {
      dispose();
    }
  };

  const signalListener = () => {
    dispose();
    pushEvent({ kind: "error", message: "cancelled" });
  };
  request.signal?.addEventListener("abort", signalListener, { once: true });
  if (request.signal?.aborted) {
    signalListener();
  }

  child.stdout.setEncoding("utf8");
  child.stderr.setEncoding("utf8");

  let stderrBuffer = "";

  child.stdout.on("data", (chunk: string) => {
    if (terminalEvent !== undefined) {
      return;
    }
    const events = parser.push(chunk);
    for (const event of events) {
      pushEvent(event);
      if (terminalEvent !== undefined) {
        break;
      }
    }
  });

  child.stderr.on("data", (chunk: string) => {
    stderrBuffer += chunk;
  });

  child.once("error", (err) => {
    pushEvent({
      kind: "error",
      message: `failed to spawn rex-cli: ${err.message}`,
    });
  });

  child.once("close", (code) => {
    if (terminalEvent === undefined) {
      const tail = parser.flush();
      for (const event of tail) {
        pushEvent(event);
        if (terminalEvent !== undefined) {
          break;
        }
      }
    }
    if (terminalEvent === undefined) {
      const stderrTrim = stderrBuffer.trim();
      const message =
        code === 0
          ? "stream ended without terminal event"
          : `rex-cli exited with code ${code}${stderrTrim.length > 0 ? `: ${stderrTrim}` : ""}`;
      pushEvent({ kind: "error", message });
    }
    wakeConsumer();
  });

  try {
    while (true) {
      if (queue.length > 0) {
        const next = queue.shift() as StreamEvent;
        yield next;
        if (next.kind === "done" || next.kind === "error") {
          return;
        }
        continue;
      }
      if (terminalEvent !== undefined && queue.length === 0) {
        return;
      }
      await new Promise<void>((resolve) => {
        pendingResolve = resolve;
      });
    }
  } finally {
    cleanup();
  }
}

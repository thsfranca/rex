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
  let terminated = false;
  let terminationEvent: StreamEvent | undefined;
  let spawnError: Error | undefined;

  const signalListener = () => {
    if (!terminated) {
      dispose();
      queue.push({ kind: "error", message: "cancelled" });
      terminated = true;
      pendingResolve?.();
    }
  };
  request.signal?.addEventListener("abort", signalListener, { once: true });

  child.stdout.setEncoding("utf8");
  child.stderr.setEncoding("utf8");

  let stderrBuffer = "";

  child.stdout.on("data", (chunk: string) => {
    if (terminated) {
      return;
    }
    const events = parser.push(chunk);
    if (events.length > 0) {
      for (const event of events) {
        queue.push(event);
        if (event.kind === "done" || event.kind === "error") {
          terminated = true;
          terminationEvent = event;
          break;
        }
      }
      pendingResolve?.();
    }
  });

  child.stderr.on("data", (chunk: string) => {
    stderrBuffer += chunk;
  });

  child.once("error", (err) => {
    spawnError = err;
    if (!terminated) {
      queue.push({
        kind: "error",
        message: `failed to spawn rex-cli: ${err.message}`,
      });
      terminated = true;
      pendingResolve?.();
    }
  });

  child.once("close", (code) => {
    if (!terminated) {
      const tail = parser.flush();
      for (const event of tail) {
        queue.push(event);
        if (event.kind === "done" || event.kind === "error") {
          terminated = true;
          terminationEvent = event;
          break;
        }
      }
    }
    if (!terminated) {
      const stderrTrim = stderrBuffer.trim();
      const message =
        code === 0
          ? "stream ended without terminal event"
          : `rex-cli exited with code ${code}${stderrTrim.length > 0 ? `: ${stderrTrim}` : ""}`;
      queue.push({ kind: "error", message });
      terminated = true;
    }
    pendingResolve?.();
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
      if (terminated && queue.length === 0) {
        if (terminationEvent !== undefined) {
          return;
        }
        if (spawnError !== undefined) {
          return;
        }
        return;
      }
      await new Promise<void>((resolve) => {
        pendingResolve = resolve;
      });
      pendingResolve = undefined;
    }
  } finally {
    request.signal?.removeEventListener("abort", signalListener);
    if (!child.killed) {
      dispose();
    }
  }
}

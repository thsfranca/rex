/**
 * Pure, dependency-free NDJSON line parser for `rex-cli --format ndjson` streams.
 *
 * The upstream contract defined in `docs/EXTENSION.md` is:
 * - one JSON object per stdout line;
 * - exactly one terminal event (`done` or `error`);
 * - events: `chunk` (`event`, `index`, `text`), `done` (`event`, `index`),
 *   `error` (`event`, `message`).
 *
 * The parser is intentionally defensive: malformed lines surface as synthetic
 * `error` events so downstream code can terminate the stream instead of
 * crashing.
 */

export interface StreamChunkEvent {
  readonly kind: "chunk";
  readonly index: number;
  readonly text: string;
}

export interface StreamDoneEvent {
  readonly kind: "done";
  readonly index: number;
}

export interface StreamErrorEvent {
  readonly kind: "error";
  readonly message: string;
  readonly code?: StreamErrorCode;
}

export type StreamEvent = StreamChunkEvent | StreamDoneEvent | StreamErrorEvent;

export type StreamErrorCode =
  | "daemon_unavailable"
  | "stream_timeout"
  | "stream_interrupted"
  | "stream_incomplete"
  | "cancelled"
  | "invalid_response"
  | "spawn_failed"
  | "unknown";

export class NdjsonLineParser {
  private buffer = "";

  /**
   * Feed a new chunk of stdout bytes (already decoded as text). Returns any
   * newly completed events in order. Partial lines are retained for the next
   * feed call.
   */
  push(input: string): StreamEvent[] {
    this.buffer += input;
    const events: StreamEvent[] = [];
    let newlineIndex = this.buffer.indexOf("\n");
    while (newlineIndex !== -1) {
      const line = this.buffer.slice(0, newlineIndex);
      this.buffer = this.buffer.slice(newlineIndex + 1);
      const event = parseLine(line);
      if (event !== undefined) {
        events.push(event);
      }
      newlineIndex = this.buffer.indexOf("\n");
    }
    return events;
  }

  /**
   * Flush any remaining buffered content (for example when the child process
   * closes stdout without a trailing newline). Returns at most one event.
   */
  flush(): StreamEvent[] {
    if (this.buffer.length === 0) {
      return [];
    }
    const remaining = this.buffer;
    this.buffer = "";
    const event = parseLine(remaining);
    return event === undefined ? [] : [event];
  }
}

function parseLine(raw: string): StreamEvent | undefined {
  const line = raw.endsWith("\r") ? raw.slice(0, -1) : raw;
  if (line.trim().length === 0) {
    return undefined;
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(line);
  } catch {
    return {
      kind: "error",
      message: `Malformed NDJSON line: ${truncate(line)}`,
    };
  }
  if (!isRecord(parsed)) {
    return {
      kind: "error",
      message: `NDJSON line is not an object: ${truncate(line)}`,
    };
  }
  const event = parsed["event"];
  if (event === "chunk") {
    const index = asFiniteNumber(parsed["index"]);
    const text = typeof parsed["text"] === "string" ? parsed["text"] : "";
    if (index === undefined) {
      return { kind: "error", message: "chunk event missing numeric index" };
    }
    return { kind: "chunk", index, text };
  }
  if (event === "done") {
    const index = asFiniteNumber(parsed["index"]) ?? 0;
    return { kind: "done", index };
  }
  if (event === "error") {
    const message =
      typeof parsed["message"] === "string" && parsed["message"].length > 0
        ? parsed["message"]
        : "unknown error";
    const code = asErrorCode(parsed["code"]);
    return { kind: "error", message, ...(code === undefined ? {} : { code }) };
  }
  return {
    kind: "error",
    message: `Unknown NDJSON event type: ${JSON.stringify(event)}`,
  };
}

function asErrorCode(value: unknown): StreamErrorCode | undefined {
  if (typeof value !== "string") {
    return undefined;
  }
  if (
    value === "daemon_unavailable" ||
    value === "stream_timeout" ||
    value === "stream_interrupted" ||
    value === "stream_incomplete" ||
    value === "cancelled" ||
    value === "invalid_response" ||
    value === "spawn_failed" ||
    value === "unknown"
  ) {
    return value;
  }
  return undefined;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function asFiniteNumber(value: unknown): number | undefined {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return undefined;
  }
  return value;
}

function truncate(input: string, max = 120): string {
  return input.length <= max ? input : `${input.slice(0, max)}...`;
}

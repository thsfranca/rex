/**
 * Pure, dependency-free NDJSON line parser for `rex-cli --format ndjson` streams.
 *
 * The upstream contract defined in `docs/EXTENSION.md` is:
 * - one JSON object per stdout line;
 * - exactly one terminal event (`done` or `error`);
 * - events: `chunk`, `done`, `error`, and additive non-terminal `tool`, `step`, `plan`.
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

export interface StreamToolEvent {
  readonly kind: "tool";
  readonly index: number;
  readonly name: string;
  readonly phase: string;
  readonly detail?: string;
}

export interface StreamStepEvent {
  readonly kind: "step";
  readonly index: number;
  readonly phase: string;
  readonly summary: string;
}

export type PlanStreamPhase = "draft" | "clarify" | "ready";

export interface StreamPlanEvent {
  readonly kind: "plan";
  readonly index: number;
  readonly phase: PlanStreamPhase;
  readonly title: string;
  readonly detail: string;
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

export type StreamEvent =
  | StreamChunkEvent
  | StreamToolEvent
  | StreamStepEvent
  | StreamPlanEvent
  | StreamDoneEvent
  | StreamErrorEvent;

export type StreamErrorCode =
  | "daemon_unavailable"
  | "sidecar_unavailable"
  | "inference_config"
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
  if (event === "tool") {
    const index = asFiniteNumber(parsed["index"]);
    const name = typeof parsed["name"] === "string" ? parsed["name"] : "";
    const phase = typeof parsed["phase"] === "string" ? parsed["phase"] : "";
    const detail = typeof parsed["detail"] === "string" ? parsed["detail"] : undefined;
    if (index === undefined || name.length === 0 || phase.length === 0) {
      return { kind: "error", message: "tool event missing required fields" };
    }
    return { kind: "tool", index, name, phase, ...(detail === undefined ? {} : { detail }) };
  }
  if (event === "step") {
    const index = asFiniteNumber(parsed["index"]);
    const phase = typeof parsed["phase"] === "string" ? parsed["phase"] : "";
    const summary = typeof parsed["summary"] === "string" ? parsed["summary"] : "";
    if (index === undefined || phase.length === 0 || summary.length === 0) {
      return { kind: "error", message: "step event missing required fields" };
    }
    return { kind: "step", index, phase, summary };
  }
  if (event === "plan") {
    const index = asFiniteNumber(parsed["index"]);
    const phaseRaw = typeof parsed["phase"] === "string" ? parsed["phase"] : "";
    const title = typeof parsed["title"] === "string" ? parsed["title"] : "";
    const detail = typeof parsed["detail"] === "string" ? parsed["detail"] : "";
    const phase = asPlanPhase(phaseRaw);
    if (index === undefined || phase === undefined || title.length === 0) {
      return { kind: "error", message: "plan event missing required fields" };
    }
    return { kind: "plan", index, phase, title, detail };
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
    value === "sidecar_unavailable" ||
    value === "inference_config" ||
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

function asPlanPhase(value: string): PlanStreamPhase | undefined {
  if (value === "draft" || value === "clarify" || value === "ready") {
    return value;
  }
  return undefined;
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

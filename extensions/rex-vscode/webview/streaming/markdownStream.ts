import { marked } from "marked";

/**
 * Incremental markdown renderer used during streaming responses.
 *
 * Re-parses the buffer only at safe boundaries (closed line *or* closed
 * fence) so the UI does not flicker while a fenced block is still growing.
 * When the buffer is inside an open fence, the renderer returns the last
 * stable render plus a trailing raw-text block so the user still sees
 * characters arrive in real time.
 */
export interface RenderResult {
  /** Fully parsed markdown HTML for the stable prefix of the buffer. */
  readonly html: string;
  /** Text that has not yet been parsed (inside an open fence). */
  readonly trailingRaw: string;
}

export class MarkdownStream {
  private buffer = "";
  private lastStableLength = 0;
  private lastStableHtml = "";

  push(text: string): RenderResult {
    if (text.length === 0) {
      return this.currentResult();
    }
    this.buffer += text;
    return this.recompute();
  }

  reset(): void {
    this.buffer = "";
    this.lastStableLength = 0;
    this.lastStableHtml = "";
  }

  finalize(): RenderResult {
    this.lastStableLength = this.buffer.length;
    this.lastStableHtml = renderMarkdown(this.buffer);
    return { html: this.lastStableHtml, trailingRaw: "" };
  }

  private currentResult(): RenderResult {
    return {
      html: this.lastStableHtml,
      trailingRaw: this.buffer.slice(this.lastStableLength),
    };
  }

  private recompute(): RenderResult {
    const boundary = findLastStableBoundary(this.buffer);
    if (boundary > this.lastStableLength) {
      const prefix = this.buffer.slice(0, boundary);
      this.lastStableHtml = renderMarkdown(prefix);
      this.lastStableLength = boundary;
    }
    return this.currentResult();
  }
}

export function findLastStableBoundary(input: string): number {
  if (input.length === 0) {
    return 0;
  }
  const fenceOpen = isInsideFence(input);
  if (fenceOpen) {
    return input.lastIndexOf("\n", input.length - 1) >= 0
      ? lastSafeIndexBeforeFence(input)
      : 0;
  }
  const lastNewline = input.lastIndexOf("\n");
  if (lastNewline === -1) {
    return 0;
  }
  return lastNewline + 1;
}

function isInsideFence(input: string): boolean {
  const fences = input.match(/^```/gm);
  if (fences === null) {
    return false;
  }
  return fences.length % 2 === 1;
}

function lastSafeIndexBeforeFence(input: string): number {
  const matches = [...input.matchAll(/^```/gm)];
  if (matches.length === 0) {
    return 0;
  }
  const lastOpen = matches[matches.length - 1];
  return lastOpen.index ?? 0;
}

export function renderMarkdown(input: string): string {
  return marked.parse(input, { async: false, gfm: true, breaks: false }) as string;
}

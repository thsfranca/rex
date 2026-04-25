export type Segment =
  | { readonly kind: "markdown"; readonly content: string }
  | { readonly kind: "code"; readonly content: string; readonly language: string };

/**
 * Split a markdown buffer into interleaved text segments and fenced code
 * blocks. Returns segments in source order and drops blank text segments so
 * the chat UI can place `CodeBlock` components directly between renders.
 */
export function splitByCodeBlocks(input: string): ReadonlyArray<Segment> {
  if (input.length === 0) {
    return [];
  }
  const segments: Segment[] = [];
  const fenceRegex = /```(\S*)\n([\s\S]*?)```/g;
  let cursor = 0;
  let match: RegExpExecArray | null = fenceRegex.exec(input);
  while (match !== null) {
    if (match.index > cursor) {
      const leading = input.slice(cursor, match.index);
      if (leading.trim().length > 0) {
        segments.push({ kind: "markdown", content: leading });
      }
    }
    segments.push({
      kind: "code",
      content: match[2] ?? "",
      language: match[1] ?? "",
    });
    cursor = fenceRegex.lastIndex;
    match = fenceRegex.exec(input);
  }
  if (cursor < input.length) {
    const trailing = input.slice(cursor);
    if (trailing.trim().length > 0) {
      segments.push({ kind: "markdown", content: trailing });
    }
  }
  return segments;
}

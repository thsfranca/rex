export type DiffLineKind = "context" | "remove" | "add";

export interface DiffLine {
  readonly kind: DiffLineKind;
  readonly text: string;
  readonly oldLineNumber?: number;
  readonly newLineNumber?: number;
}

const MAX_DIFF_LINES = 160;

export function computeLineDiff(before: string, after: string): DiffLine[] {
  const oldLines = splitLines(before);
  const newLines = splitLines(after);
  const lcs = buildLcsTable(oldLines, newLines);
  const raw = backtrackDiff(oldLines, newLines, lcs);
  return trimDiffForDisplay(raw);
}

function splitLines(text: string): string[] {
  if (text.length === 0) {
    return [];
  }
  return text.replace(/\r\n/g, "\n").split("\n");
}

function buildLcsTable(a: string[], b: string[]): number[][] {
  const rows = a.length + 1;
  const cols = b.length + 1;
  const table: number[][] = Array.from({ length: rows }, () => Array<number>(cols).fill(0));
  for (let i = 1; i < rows; i += 1) {
    for (let j = 1; j < cols; j += 1) {
      if (a[i - 1] === b[j - 1]) {
        table[i][j] = table[i - 1][j - 1] + 1;
      } else {
        table[i][j] = Math.max(table[i - 1][j], table[i][j - 1]);
      }
    }
  }
  return table;
}

function backtrackDiff(a: string[], b: string[], table: number[][]): DiffLine[] {
  const result: DiffLine[] = [];
  let i = a.length;
  let j = b.length;
  while (i > 0 || j > 0) {
    if (i > 0 && j > 0 && a[i - 1] === b[j - 1]) {
      result.push({
        kind: "context",
        text: a[i - 1] ?? "",
        oldLineNumber: i,
        newLineNumber: j,
      });
      i -= 1;
      j -= 1;
      continue;
    }
    if (j > 0 && (i === 0 || table[i][j - 1] >= table[i - 1][j])) {
      result.push({
        kind: "add",
        text: b[j - 1] ?? "",
        newLineNumber: j,
      });
      j -= 1;
      continue;
    }
    if (i > 0) {
      result.push({
        kind: "remove",
        text: a[i - 1] ?? "",
        oldLineNumber: i,
      });
      i -= 1;
    }
  }
  return result.reverse();
}

function trimDiffForDisplay(lines: DiffLine[]): DiffLine[] {
  if (lines.length <= MAX_DIFF_LINES) {
    return lines;
  }
  const head = lines.slice(0, MAX_DIFF_LINES - 1);
  head.push({ kind: "context", text: "… diff truncated …" });
  return head;
}

export function diffStats(lines: ReadonlyArray<DiffLine>): { added: number; removed: number } {
  let added = 0;
  let removed = 0;
  for (const line of lines) {
    if (line.kind === "add") {
      added += 1;
    } else if (line.kind === "remove") {
      removed += 1;
    }
  }
  return { added, removed };
}

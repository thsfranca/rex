export interface ExecutionLabelInput {
  readonly summary: string;
  readonly phase: string;
  readonly kind?: string;
  readonly detail?: string;
  /** Path, command, or query captured while the tool was running. */
  readonly target?: string;
}

const NOISE_STEP_SUMMARIES = new Set([
  "Execution started.",
  "Execution completed.",
  "Approval granted.",
  "Approval denied.",
]);

export function isTimelineNoise(entry: ExecutionLabelInput): boolean {
  if (entry.kind === "tool" || entry.kind === "activity") {
    return false;
  }
  if (entry.kind !== "step" && entry.kind !== undefined) {
    return false;
  }
  const summary = entry.summary.trim();
  if (NOISE_STEP_SUMMARIES.has(summary)) {
    return true;
  }
  if (summary.startsWith("Request queued in ") && summary.endsWith(" mode.")) {
    return true;
  }
  if (/ invoking /.test(summary) && summary.endsWith(" tools")) {
    return true;
  }
  if (/ invoking fs\.| invoking exec\.| invoking web\./.test(summary)) {
    return true;
  }
  if (summary.startsWith("Execution failed:")) {
    return false;
  }
  return entry.phase === "running" || entry.phase === "completed" || entry.phase === "queued";
}

function basenamePath(path: string): string {
  const normalized = path.replace(/\\/g, "/").trim();
  const segments = normalized.split("/").filter((segment) => segment.length > 0);
  return segments.length > 0 ? (segments[segments.length - 1] ?? path) : path;
}

function truncate(text: string, max: number): string {
  const trimmed = text.trim();
  if (trimmed.length <= max) {
    return trimmed;
  }
  return `${trimmed.slice(0, max - 1)}…`;
}

function titleCaseWords(text: string): string {
  return text
    .split(/[\s_.-]+/)
    .filter((part) => part.length > 0)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1).toLowerCase())
    .join(" ");
}

export function normalizeToolName(raw: string): string {
  const trimmed = raw.trim().toLowerCase();
  if (trimmed.length === 0) {
    return trimmed;
  }
  return trimmed.replace(/\s+/g, "_");
}

type VerbPair = { running: string; done: string; failed: string };

const TOOL_VERBS: Record<string, VerbPair> = {
  read_file: { running: "Reading", done: "Read", failed: "Failed to read" },
  read: { running: "Reading", done: "Read", failed: "Failed to read" },
  "fs.read": { running: "Reading", done: "Read", failed: "Failed to read" },
  write_file: { running: "Writing", done: "Wrote", failed: "Failed to write" },
  write: { running: "Writing", done: "Wrote", failed: "Failed to write" },
  "fs.write": { running: "Writing", done: "Wrote", failed: "Failed to write" },
  edit_file: { running: "Editing", done: "Edited", failed: "Failed to edit" },
  search_replace: { running: "Editing", done: "Edited", failed: "Failed to edit" },
  apply_patch: { running: "Applying patch to", done: "Applied patch to", failed: "Failed to patch" },
  run_terminal_cmd: { running: "Running", done: "Ran", failed: "Failed to run" },
  shell: { running: "Running", done: "Ran", failed: "Failed to run" },
  bash: { running: "Running", done: "Ran", failed: "Failed to run" },
  terminal: { running: "Running", done: "Ran", failed: "Failed to run" },
  "exec.shell": { running: "Running", done: "Ran", failed: "Failed to run" },
  grep: { running: "Searching for", done: "Searched for", failed: "Search failed for" },
  search: { running: "Searching", done: "Searched", failed: "Search failed" },
  codebase_search: { running: "Searching codebase for", done: "Searched codebase for", failed: "Search failed for" },
  list_dir: { running: "Listing", done: "Listed", failed: "Failed to list" },
  "fs.list": { running: "Listing", done: "Listed", failed: "Failed to list" },
  glob_file_search: { running: "Finding files matching", done: "Found files matching", failed: "File search failed for" },
  delete_file: { running: "Deleting", done: "Deleted", failed: "Failed to delete" },
  web_search: { running: "Searching the web for", done: "Searched the web for", failed: "Web search failed for" },
  "web.search": { running: "Searching the web for", done: "Searched the web for", failed: "Web search failed for" },
  web_fetch: { running: "Fetching", done: "Fetched", failed: "Failed to fetch" },
  "web.fetch": { running: "Fetching", done: "Fetched", failed: "Failed to fetch" },
  plan_save: { running: "Saving plan", done: "Saved plan", failed: "Failed to save plan" },
  "plan.save": { running: "Saving plan", done: "Saved plan", failed: "Failed to save plan" },
};

function pickVerb(toolName: string, phase: string): string {
  const normalized = normalizeToolName(toolName);
  const verbs = TOOL_VERBS[normalized];
  if (verbs === undefined) {
    const fallback = titleCaseWords(normalized);
    if (phase === "failed") {
      return `Failed: ${fallback}`;
    }
    return fallback;
  }
  if (phase === "failed") {
    return verbs.failed;
  }
  if (phase === "completed") {
    return verbs.done;
  }
  return verbs.running;
}

export function stripToolResultDelimiters(text: string): string {
  const trimmed = text.trim();
  const match = trimmed.match(/^<<TOOL_RESULT:[^>]+>>\n([\s\S]*?)\n<<END>>\s*$/);
  if (match !== null) {
    return match[1] ?? trimmed;
  }
  return trimmed;
}

export function isToolResultBody(detail: string): boolean {
  const trimmed = detail.trim();
  if (trimmed.length === 0) {
    return false;
  }
  if (trimmed.startsWith("<<TOOL_RESULT:")) {
    return true;
  }
  if (trimmed.startsWith("[cached read of ")) {
    return true;
  }
  if (trimmed.startsWith("[tool ")) {
    return true;
  }
  if (trimmed.startsWith("[pruned read:")) {
    return true;
  }
  if (trimmed.includes("\n<<END>>")) {
    return true;
  }
  const normalized = stripToolResultDelimiters(trimmed);
  if (normalized !== trimmed) {
    return true;
  }
  if (/^[^,\n]+(?:,\s*[^,\n]+){2,}/.test(normalized)) {
    return true;
  }
  if (normalized.startsWith("# ") || normalized.startsWith("## ")) {
    return true;
  }
  return normalized.length > 120;
}

export function extractTargetFromResult(toolName: string, detail: string): string | undefined {
  const normalized = normalizeToolName(toolName);
  const trimmed = detail.trim();
  if (trimmed.length === 0) {
    return undefined;
  }

  const cached = trimmed.match(/^\[cached read of ([^\]]+)\]/);
  if (cached !== null) {
    return cached[1]?.trim();
  }

  if (isToolResultBody(trimmed)) {
    return undefined;
  }

  if (normalized === "fs.list" || normalized === "list_dir") {
    if (trimmed.includes(",")) {
      return undefined;
    }
  }

  if (normalized === "exec.shell" || normalized === "run_terminal_cmd") {
    return trimmed;
  }

  if (trimmed.includes("/") || trimmed.includes("\\") || /^[\w.-]+\.[a-z0-9]+$/i.test(trimmed)) {
    return trimmed;
  }

  return undefined;
}

export function resolveTimelineTarget(
  summary: string,
  phase: string,
  kind: string | undefined,
  detail: string | undefined,
  previousTarget?: string,
): string | undefined {
  if (kind !== "tool") {
    return previousTarget;
  }

  const trimmedDetail = detail?.trim() ?? "";

  if (phase === "running") {
    if (trimmedDetail.length > 0 && !isToolResultBody(trimmedDetail)) {
      return trimmedDetail;
    }
    return previousTarget;
  }

  if (previousTarget !== undefined && previousTarget.length > 0) {
    return previousTarget;
  }

  return extractTargetFromResult(summary, trimmedDetail);
}

function formatDirectoryTarget(target: string | undefined): string | undefined {
  const trimmed = target?.trim() ?? "";
  if (trimmed.length === 0 || trimmed === "." || trimmed === "./") {
    return undefined;
  }
  return trimmed.includes("/") || trimmed.includes("\\") ? basenamePath(trimmed) : trimmed;
}

function countListEntries(detail: string): number | undefined {
  const body = stripToolResultDelimiters(detail).trim();
  if (body.length === 0) {
    return undefined;
  }
  if (body === "(empty)") {
    return 0;
  }
  if (!body.includes(",")) {
    return 1;
  }
  return body.split(",").filter((part) => part.trim().length > 0).length;
}

function formatListLabel(input: ExecutionLabelInput, verb: string): string {
  const dir = formatDirectoryTarget(input.target);
  const where = dir !== undefined ? ` ${dir}` : " workspace";

  if (input.phase === "completed" && input.detail !== undefined && isToolResultBody(input.detail)) {
    const count = countListEntries(input.detail);
    if (count !== undefined) {
      const suffix = dir !== undefined ? ` in ${dir}` : "";
      return `Listed ${count} item${count === 1 ? "" : "s"}${suffix}`;
    }
  }

  return `${verb}${where}`;
}

function formatToolLabel(input: ExecutionLabelInput): string {
  const toolName = normalizeToolName(input.summary);
  const verb = pickVerb(toolName, input.phase);
  const target = input.target?.trim() ?? "";

  if (toolName === "fs.list" || toolName === "list_dir") {
    return formatListLabel(input, verb);
  }

  const isCommandTool =
    toolName === "run_terminal_cmd" ||
    toolName === "shell" ||
    toolName === "bash" ||
    toolName === "terminal" ||
    toolName === "exec.shell";

  const subjectSource =
    target.length > 0
      ? target
      : extractTargetFromResult(toolName, input.detail?.trim() ?? "") ?? "";

  if (isCommandTool) {
    const command = subjectSource.length > 0 ? subjectSource : toolName;
    return `${verb} ${truncate(command, 96)}`;
  }

  if (toolName === "web.search" || toolName === "web_search") {
    if (subjectSource.length > 0) {
      return `${verb} ${truncate(subjectSource, 72)}`;
    }
    return verb;
  }

  if (subjectSource.length > 0) {
    const subject =
      subjectSource.includes("/") || subjectSource.includes("\\")
        ? basenamePath(subjectSource)
        : truncate(subjectSource, 72);
    return `${verb} ${subject}`;
  }

  const rawDetail = input.detail?.trim() ?? "";
  if (rawDetail.length > 0 && !isToolResultBody(rawDetail)) {
    return `${verb} ${truncate(rawDetail, 72)}`;
  }

  return verb;
}

function formatActivityLabel(input: ExecutionLabelInput): string {
  const summary = input.summary.trim();
  if (/^[a-z][a-z0-9_.]*$/i.test(summary)) {
    const normalized = normalizeToolName(summary);
    if (TOOL_VERBS[normalized] !== undefined) {
      return formatToolLabel({ ...input, summary: normalized, kind: "tool" });
    }
    if (summary.includes("_") || summary.includes(".")) {
      return titleCaseWords(summary);
    }
  }
  return summary;
}

function formatStepLabel(input: ExecutionLabelInput): string {
  const summary = input.summary.trim();
  if (input.phase === "awaiting_approval") {
    return summary;
  }
  if (input.phase === "blocked") {
    return summary.length > 0 ? summary : "Blocked";
  }
  if (input.phase === "cancelled") {
    return summary.length > 0 ? summary : "Cancelled";
  }
  if (input.phase === "failed") {
    return summary;
  }
  return summary;
}

export function formatExecutionLabel(input: ExecutionLabelInput): string {
  if (input.kind === "activity") {
    return formatActivityLabel(input);
  }
  if (input.kind === "tool") {
    return formatToolLabel(input);
  }
  return formatStepLabel(input);
}

export function shouldShowExecutionDetail(
  summary: string,
  detail: string | undefined,
  target?: string,
): boolean {
  if (detail === undefined || detail.trim().length === 0) {
    return false;
  }
  const trimmed = detail.trim();
  if (target !== undefined && trimmed === target.trim()) {
    return false;
  }
  return isToolResultBody(trimmed);
}

export interface ExecutionLabelInput {
  readonly summary: string;
  readonly phase: string;
  readonly kind?: string;
  readonly detail?: string;
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
  if (NOISE_STEP_SUMMARIES.has(entry.summary)) {
    return true;
  }
  if (entry.summary.startsWith("Request queued in ") && entry.summary.endsWith(" mode.")) {
    return true;
  }
  if (entry.summary.startsWith("Execution failed:")) {
    return false;
  }
  return entry.phase === "running" || entry.phase === "completed" || entry.phase === "queued";
}

function basename(path: string): string {
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
    .split(/[\s_-]+/)
    .filter((part) => part.length > 0)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1).toLowerCase())
    .join(" ");
}

type VerbPair = { running: string; done: string; failed: string };

const TOOL_VERBS: Record<string, VerbPair> = {
  read_file: { running: "Reading", done: "Read", failed: "Failed to read" },
  read: { running: "Reading", done: "Read", failed: "Failed to read" },
  write_file: { running: "Writing", done: "Wrote", failed: "Failed to write" },
  write: { running: "Writing", done: "Wrote", failed: "Failed to write" },
  edit_file: { running: "Editing", done: "Edited", failed: "Failed to edit" },
  search_replace: { running: "Editing", done: "Edited", failed: "Failed to edit" },
  apply_patch: { running: "Applying patch to", done: "Applied patch to", failed: "Failed to patch" },
  run_terminal_cmd: { running: "Running", done: "Ran", failed: "Failed to run" },
  shell: { running: "Running", done: "Ran", failed: "Failed to run" },
  bash: { running: "Running", done: "Ran", failed: "Failed to run" },
  terminal: { running: "Running", done: "Ran", failed: "Failed to run" },
  grep: { running: "Searching for", done: "Searched for", failed: "Search failed for" },
  search: { running: "Searching", done: "Searched", failed: "Search failed" },
  codebase_search: { running: "Searching codebase for", done: "Searched codebase for", failed: "Search failed for" },
  list_dir: { running: "Listing", done: "Listed", failed: "Failed to list" },
  glob_file_search: { running: "Finding files matching", done: "Found files matching", failed: "File search failed for" },
  delete_file: { running: "Deleting", done: "Deleted", failed: "Failed to delete" },
  web_search: { running: "Searching the web for", done: "Searched the web for", failed: "Web search failed for" },
  web_fetch: { running: "Fetching", done: "Fetched", failed: "Failed to fetch" },
};

function pickVerb(toolName: string, phase: string): string {
  const verbs = TOOL_VERBS[toolName];
  if (verbs === undefined) {
    const fallback = titleCaseWords(toolName);
    if (phase === "failed") {
      return `Failed: ${fallback}`;
    }
    if (phase === "completed") {
      return fallback;
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

function formatToolLabel(input: ExecutionLabelInput): string {
  const toolName = input.summary.trim();
  const detail = input.detail?.trim() ?? "";
  const verb = pickVerb(toolName, input.phase);

  const isCommandTool =
    toolName === "run_terminal_cmd" ||
    toolName === "shell" ||
    toolName === "bash" ||
    toolName === "terminal";

  if (isCommandTool) {
    const command = detail.length > 0 ? detail : toolName;
    return `${verb} ${truncate(command, 96)}`;
  }

  if (detail.length > 0) {
    const subject =
      detail.includes("/") || detail.includes("\\") ? basename(detail) : truncate(detail, 72);
    return `${verb} ${subject}`;
  }

  return verb;
}

function formatActivityLabel(input: ExecutionLabelInput): string {
  const summary = input.summary.trim();
  if (/^[a-z][a-z0-9_]*$/i.test(summary) && summary.includes("_")) {
    return titleCaseWords(summary);
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

export function shouldShowExecutionDetail(summary: string, detail: string | undefined): boolean {
  if (detail === undefined || detail.trim().length === 0) {
    return false;
  }
  const trimmed = detail.trim();
  const toolName = summary.trim();

  const isCommandTool =
    toolName === "run_terminal_cmd" ||
    toolName === "shell" ||
    toolName === "bash" ||
    toolName === "terminal";

  if (isCommandTool) {
    return trimmed.length > 48;
  }

  if ((trimmed.includes("/") || trimmed.includes("\\")) && basename(trimmed) !== trimmed) {
    return true;
  }

  return false;
}

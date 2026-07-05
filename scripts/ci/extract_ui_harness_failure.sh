#!/usr/bin/env bash
# Extract UI harness failure context from ui-harness.log for CI annotations.
set -euo pipefail

logfile="${1:-}"
max_lines="${2:-40}"

if [[ -z "${logfile}" ]]; then
  echo "(no ui harness log path provided)"
  exit 0
fi

if [[ ! -f "${logfile}" ]]; then
  echo "(ui harness log missing: ${logfile})"
  exit 0
fi

strip_ansi() {
  sed 's/\x1b\[[0-9;]*m//g'
}

emit_lines() {
  local content="$1"
  if [[ -n "${content}" ]]; then
    printf '%s\n' "${content}"
    exit 0
  fi
}

if harness="$(
  grep -E 'UI_HARNESS_(FAIL|ERROR|DETAIL)|Timed out waiting|TimeoutError|page\.waitFor' "${logfile}" 2>/dev/null \
    | strip_ansi \
    | tail -n "${max_lines}"
)" && [[ -n "${harness}" ]]; then
  emit_lines "${harness}"
fi

if command -v node >/dev/null 2>&1; then
  if json_excerpt="$(
    node - "${logfile}" "${max_lines}" <<'NODE'
const fs = require("node:fs");

const logPath = process.argv[2];
const maxLines = Number.parseInt(process.argv[3] ?? "40", 10);
const log = fs.readFileSync(logPath, "utf8");
const blocks = [...log.matchAll(/\{\s*"mode"[\s\S]*?\n\}/g)];
const last = blocks.length ? blocks.at(-1)[0] : null;
if (!last) {
  process.exit(2);
}

let parsed;
try {
  parsed = JSON.parse(last);
} catch {
  process.exit(2);
}

const failed = (parsed.steps ?? []).filter((step) => step.pass === false);
const lines = [];
for (const step of failed) {
  lines.push(`UI_HARNESS_FAIL step=${JSON.stringify(step.step)}`);
  if (step.detail !== undefined) {
    lines.push(`UI_HARNESS_DETAIL ${JSON.stringify(step.detail)}`);
  }
}
if (lines.length === 0 && parsed.pass === false) {
  lines.push("UI_HARNESS_FAIL harness reported pass=false with no failed steps");
}
if (lines.length === 0) {
  process.exit(2);
}
console.log(lines.slice(-maxLines).join("\n"));
NODE
  )"; then
    emit_lines "${json_excerpt}"
  fi
fi

if errors="$(
  grep -n -E 'Error:|^error:|AssertionError|panicked at|Process completed with exit code' "${logfile}" 2>/dev/null \
    | strip_ansi \
    | tail -n "${max_lines}"
)" && [[ -n "${errors}" ]]; then
  emit_lines "${errors}"
fi

echo "--- tail of ${logfile} (no ui harness failure pattern matched) ---"
tail -n "${max_lines}" "${logfile}" | strip_ansi

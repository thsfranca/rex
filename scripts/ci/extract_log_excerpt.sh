#!/usr/bin/env bash
# Print failure-relevant lines from a CI log (for annotations and summaries).
set -euo pipefail

logfile="${1:-}"
max_lines="${2:-40}"

if [[ -z "${logfile}" ]]; then
  echo "(no log file path provided)"
  exit 0
fi

if [[ ! -f "${logfile}" ]]; then
  echo "(log file missing: ${logfile})"
  exit 0
fi

strip_ansi() {
  sed 's/\x1b\[[0-9;]*m//g'
}

failure_pattern='FAIL \[|FAILED\.| failures:|panicked at|error\[E[0-9]+|error: |^Error:|assertion `|assertion failed|could not compile|CI_SIGNAL|::error::|RUST_BACKTRACE|exit code:|Process completed with exit code'

if matches="$(grep -n -E "${failure_pattern}" "${logfile}" 2>/dev/null | strip_ansi | tail -n "${max_lines}")" && [[ -n "${matches}" ]]; then
  printf '%s\n' "${matches}"
  exit 0
fi

echo "--- tail of ${logfile} (no failure pattern matched) ---"
tail -n "${max_lines}" "${logfile}" | strip_ansi

#!/usr/bin/env bash
# Validates NDJSON contract fixtures have exactly one terminal event (done or error).
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
NDJSON_DIR="${ROOT_DIR}/fixtures/stream_events"

failures=0

note_fail() {
  echo "::error::${1}"
  failures=$((failures + 1))
}

terminal_count() {
  local file="$1"
  grep -c '"event"[[:space:]]*:[[:space:]]*"\(done\|error\)"' "${file}" 2>/dev/null || echo 0
}

echo "::group::NDJSON terminal event invariant"
if [ ! -d "${NDJSON_DIR}" ]; then
  note_fail "Missing fixtures directory: ${NDJSON_DIR}"
else
  shopt -s nullglob
  fixtures=("${NDJSON_DIR}"/*.ndjson)
  shopt -u nullglob
  if [ "${#fixtures[@]}" -eq 0 ]; then
    note_fail "No NDJSON fixtures found under ${NDJSON_DIR}"
  fi
  for fixture in "${fixtures[@]}"; do
    # Standalone error catalog lines (no stream deltas) are validated elsewhere.
    if ! grep -qE '"event"[[:space:]]*:[[:space:]]*"(chunk|tool|step|plan)"' "${fixture}"; then
      continue
    fi
    count="$(terminal_count "${fixture}")"
    if [ "${count}" -ne 1 ]; then
      note_fail "${fixture} must have exactly one terminal event (found ${count})"
    fi
  done
fi
echo "::endgroup::"

if [ "${failures}" -gt 0 ]; then
  echo "CI_SIGNAL code=GUIDELINES_FAIL stage=NdjsonTerminal result=failure hint=${failures}_terminal_invariant"
  exit 1
fi

echo "::notice::NDJSON terminal event checks passed."

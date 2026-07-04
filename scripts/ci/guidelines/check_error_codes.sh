#!/usr/bin/env bash
# Validates NDJSON stream error codes stay in sync across yaml, docs, and fixtures.
# Does not require a rex-cli CliError mapping: the public NDJSON emit path (rex complete)
# was removed; codes remain a stream-contract catalog for TUI/fixtures.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CATALOG="${ROOT_DIR}/fixtures/guidelines/error_codes.yaml"
HUB_DOC="${ROOT_DIR}/docs/ERROR_HANDLING.md"
NDJSON_DIR="${ROOT_DIR}/fixtures/ndjson_contract"

failures=0

note_fail() {
  echo "::error::${1}"
  failures=$((failures + 1))
}

yaml_codes() {
  awk '/^  - code: / { print $3 }' "${CATALOG}" | sort -u
}

code_in_list() {
  local needle="$1"
  shift
  local item
  for item in "$@"; do
    if [ "${item}" = "${needle}" ]; then
      return 0
    fi
  done
  return 1
}

echo "::group::Error code catalog sync"
echo "::notice::Checking fixtures/guidelines/error_codes.yaml against docs."

yaml_list=()
while IFS= read -r line; do
  [ -n "${line}" ] && yaml_list+=("${line}")
done < <(yaml_codes)

for code in "${yaml_list[@]}"; do
  if ! grep -q "\`${code}\`" "${HUB_DOC}"; then
    note_fail "Code '${code}' in error_codes.yaml missing from docs/ERROR_HANDLING.md catalog table"
  fi
done

echo "::endgroup::"

echo "::group::NDJSON fixture error codes"
if [ -d "${NDJSON_DIR}" ]; then
  while IFS= read -r line; do
    [ -z "${line}" ] && continue
    code="$(printf '%s' "${line}" | sed -n 's/.*"code"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')"
    if [ -z "${code}" ]; then
      continue
    fi
    if ! code_in_list "${code}" "${yaml_list[@]}"; then
      note_fail "NDJSON fixture uses unregistered error code '${code}': ${line}"
    fi
  done < <(grep -h '"event"[[:space:]]*:[[:space:]]*"error"' "${NDJSON_DIR}"/*.ndjson 2>/dev/null || true)
else
  echo "::notice::No ndjson_contract fixtures directory; skipping fixture check."
fi
echo "::endgroup::"

if [ "${failures}" -gt 0 ]; then
  echo "CI_SIGNAL code=GUIDELINES_FAIL stage=ErrorCodes result=failure hint=${failures}_catalog_sync_mismatch"
  exit 1
fi

echo "::notice::Error code catalog checks passed."

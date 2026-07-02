#!/usr/bin/env bash
# Validates additive plan NDJSON events match the extension stream contract.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
NDJSON_DIR="${ROOT_DIR}/fixtures/ndjson_contract"
HUB_DOC="${ROOT_DIR}/docs/NDJSON_STREAM.md"

failures=0

note_fail() {
  echo "::error::${1}"
  failures=$((failures + 1))
}

echo "::group::NDJSON plan event contract"
if ! grep -q '`plan`' "${HUB_DOC}"; then
  note_fail "docs/NDJSON_STREAM.md must document additive plan NDJSON events"
fi

if [ ! -d "${NDJSON_DIR}" ]; then
  note_fail "Missing fixtures directory: ${NDJSON_DIR}"
else
  while IFS= read -r line; do
    [ -z "${line}" ] && continue
    if ! printf '%s' "${line}" | grep -q '"event"[[:space:]]*:[[:space:]]*"plan"'; then
      continue
    fi
    for field in index phase title detail; do
      if ! printf '%s' "${line}" | grep -q "\"${field}\""; then
        note_fail "plan event missing '${field}': ${line}"
      fi
    done
    phase="$(printf '%s' "${line}" | sed -n 's/.*"phase"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')"
    case "${phase}" in
      draft|clarify|ready) ;;
      *)
        note_fail "plan event has invalid phase '${phase}': ${line}"
        ;;
    esac
  done < <(grep -h '"event"[[:space:]]*:[[:space:]]*"plan"' "${NDJSON_DIR}"/*.ndjson 2>/dev/null || true)
fi
echo "::endgroup::"

if [ "${failures}" -gt 0 ]; then
  echo "CI_SIGNAL code=GUIDELINES_FAIL stage=NdjsonPlanContract result=failure hint=${failures}_plan_contract"
  exit 1
fi

echo "::notice::NDJSON plan contract checks passed."

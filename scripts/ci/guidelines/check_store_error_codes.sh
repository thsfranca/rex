#!/usr/bin/env bash
# Validates economics store error codes stay in sync across yaml, docs, and rex-obs-store.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CATALOG="${ROOT_DIR}/fixtures/guidelines/store_error_codes.yaml"
HUB_DOC="${ROOT_DIR}/docs/ERROR_HANDLING.md"
ERROR_RS="${ROOT_DIR}/crates/rex-obs-store/src/error.rs"

failures=0

note_fail() {
  echo "::error::${1}"
  failures=$((failures + 1))
}

yaml_codes() {
  awk '/^  - code: / { print $3 }' "${CATALOG}" | sort -u
}

echo "::group::Store error code catalog sync"
echo "::notice::Checking fixtures/guidelines/store_error_codes.yaml against docs and error.rs."

yaml_list=()
while IFS= read -r line; do
  [ -n "${line}" ] && yaml_list+=("${line}")
done < <(yaml_codes)

for code in "${yaml_list[@]}"; do
  if ! grep -q "\`${code}\`" "${HUB_DOC}"; then
    note_fail "Code '${code}' in store_error_codes.yaml missing from docs/ERROR_HANDLING.md store table"
  fi
  if ! grep -q "\"${code}\"" "${ERROR_RS}"; then
    note_fail "Code '${code}' in store_error_codes.yaml missing from crates/rex-obs-store/src/error.rs"
  fi
done
echo "::endgroup::"

if [ "${failures}" -gt 0 ]; then
  echo "CI_SIGNAL code=GUIDELINES_FAIL stage=StoreErrorCodes result=failure hint=${failures}_store_catalog_sync"
  exit 1
fi

echo "::notice::Store error code catalog checks passed."

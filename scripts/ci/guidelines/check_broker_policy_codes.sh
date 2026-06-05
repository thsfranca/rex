#!/usr/bin/env bash
# Validates broker policy error codes stay in sync across yaml, docs, and daemon policy.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CATALOG="${ROOT_DIR}/fixtures/guidelines/broker_error_codes.yaml"
HUB_DOC="${ROOT_DIR}/docs/ERROR_HANDLING.md"
POLICY_RS="${ROOT_DIR}/crates/rex-daemon/src/access_policy.rs"

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

echo "::group::Broker policy code catalog sync"
echo "::notice::Checking fixtures/guidelines/broker_error_codes.yaml against docs and access_policy.rs."

yaml_list=()
while IFS= read -r line; do
  [ -n "${line}" ] && yaml_list+=("${line}")
done < <(yaml_codes)

for code in "${yaml_list[@]}"; do
  if ! grep -q "\`${code}\`" "${HUB_DOC}"; then
    note_fail "Code '${code}' in broker_error_codes.yaml missing from docs/ERROR_HANDLING.md broker table"
  fi
  if ! grep -q "\"${code}\"" "${POLICY_RS}"; then
    note_fail "Code '${code}' in broker_error_codes.yaml missing from access_policy.rs"
  fi
done
echo "::endgroup::"

if [ "${failures}" -gt 0 ]; then
  echo "CI_SIGNAL code=GUIDELINES_FAIL stage=BrokerPolicyCodes result=failure hint=${failures}_broker_catalog_sync"
  exit 1
fi

echo "::notice::Broker policy code catalog checks passed."

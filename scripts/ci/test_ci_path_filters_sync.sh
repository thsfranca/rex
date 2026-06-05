#!/usr/bin/env bash
# Ensures .github/ci-path-filters.yaml stays aligned with detect-ci-changes action filters.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CANONICAL="${ROOT_DIR}/.github/ci-path-filters.yaml"
ACTION="${ROOT_DIR}/.github/actions/detect-ci-changes/action.yml"

normalize_filters() {
  awk '
    /^[[:space:]]*#/ { next }
    /^[[:space:]]*$/ { next }
    /^[[:space:]]*[a-z_]+:$/ { sub(/^[[:space:]]+/, ""); print; next }
    /^[[:space:]]+- / { sub(/^[[:space:]]+/, "  "); print }
  '
}

canonical_norm="$(normalize_filters < "${CANONICAL}")"
action_norm="$(awk '
  /^[[:space:]]*filters: \|/ { in_filters=1; next }
  in_filters && /^[[:space:]]{4}- name:/ { in_filters=0; next }
  in_filters && /^[[:space:]]+[a-z_]+:$/ { sub(/^[[:space:]]+/, ""); print; next }
  in_filters && /^[[:space:]]+- / { sub(/^[[:space:]]+/, "  "); print; next }
' "${ACTION}")"

if [ "${canonical_norm}" != "${action_norm}" ]; then
  echo "Path filter drift between ${CANONICAL} and ${ACTION}."
  diff -u <(printf '%s\n' "${canonical_norm}") <(printf '%s\n' "${action_norm}") || true
  exit 1
fi

echo "ci path filter sync check passed."

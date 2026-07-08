#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXTRACT="${SCRIPT_DIR}/extract_ui_harness_failure.sh"

tmp="$(mktemp -d)"
trap 'rm -rf "${tmp}"' EXIT

cat >"${tmp}/harness.log" <<'EOF'
 UI_HARNESS_FAIL step="assert_motion #status-dot"
 UI_HARNESS_DETAIL {"motionTier":"none"}
 {
  "mode": "desktop",
  "pass": false,
  "steps": [
    { "step": "open desktop", "pass": true },
    { "step": "assert_motion #status-dot", "pass": false, "detail": { "motionTier": "none" } }
  ]
}
EOF

out="$("${EXTRACT}" "${tmp}/harness.log")"
if [[ "${out}" != *'UI_HARNESS_FAIL step="assert_motion #status-dot"'* ]]; then
  echo "extract_ui_harness_failure did not surface harness fail lines:"
  printf '%s\n' "${out}"
  exit 1
fi

cat >"${tmp}/json-only.log" <<'EOF'
{
  "mode": "desktop",
  "pass": false,
  "steps": [
    { "step": "wait transcript hello", "pass": false }
  ]
}
EOF

out="$("${EXTRACT}" "${tmp}/json-only.log")"
if [[ "${out}" != *'UI_HARNESS_FAIL step="wait transcript hello"'* ]]; then
  echo "extract_ui_harness_failure did not parse failed JSON steps:"
  printf '%s\n' "${out}"
  exit 1
fi

echo "extract_ui_harness_failure contract tests passed."

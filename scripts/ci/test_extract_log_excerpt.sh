#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXTRACT="${SCRIPT_DIR}/extract_log_excerpt.sh"

tmp="$(mktemp -d)"
trap 'rm -rf "${tmp}"' EXIT

cat >"${tmp}/nextest.log" <<'EOF'
    Finished test profile [default] target(s) in 0.12s
        PASS [   0.001s] rex-config::merge test_host
        FAIL [   0.030s] rex-daemon::capability_fleet_smoke capability_fleet_spawns_mock_and_passes_health
    thread 'capability_fleet_spawns_mock_and_passes_health' panicked at crates/rex-daemon/tests/capability_fleet_smoke.rs:42:5:
    assertion failed: health_ok
    test result: FAILED. 1 passed; 1 failed; 0 ignored
EOF

out="$("${EXTRACT}" "${tmp}/nextest.log")"
if [[ "${out}" != *"FAIL ["* ]] || [[ "${out}" != *"panicked at"* ]]; then
  echo "extract_log_excerpt did not surface nextest failure lines:"
  printf '%s\n' "${out}"
  exit 1
fi

printf '%s\n' "(no log file path provided)" >"${tmp}/empty.out"
if [[ "$("${EXTRACT}")" != "(no log file path provided)" ]]; then
  echo "expected message for missing log path"
  exit 1
fi

cat >"${tmp}/harness.log" <<'EOF'
UI_HARNESS_FAIL step="assert_motion #status-dot"
EOF

out="$("${EXTRACT}" "${tmp}/harness.log")"
if [[ "${out}" != *"UI_HARNESS_FAIL"* ]]; then
  echo "extract_log_excerpt did not surface UI harness failure lines:"
  printf '%s\n' "${out}"
  exit 1
fi

echo "extract_log_excerpt contract tests passed."

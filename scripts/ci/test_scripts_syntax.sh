#!/usr/bin/env bash
# Fail if any scripts/**/*.sh has invalid bash syntax (bash -n).
# Also rejects `name {` function definitions (missing `()`), which parse as
# command invocations and break install scripts at runtime.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${ROOT_DIR}"

fail=0
while IFS= read -r -d '' script; do
  if ! bash -n "${script}" 2>/tmp/rex-bash-n.err; then
    echo "bash -n failed: ${script}"
    cat /tmp/rex-bash-n.err
    fail=1
  fi
  if grep -nE '^[a-zA-Z_][a-zA-Z0-9_]*[[:space:]]+\{' "${script}" >/tmp/rex-fn-style.err; then
    echo "invalid function style (use name() {): ${script}"
    cat /tmp/rex-fn-style.err
    fail=1
  fi
done < <(find scripts -type f -name '*.sh' -print0 | sort -z)

if [[ "${fail}" -ne 0 ]]; then
  echo "scripts syntax check failed."
  exit 1
fi

echo "scripts syntax check passed."

#!/usr/bin/env bash
# Runs all executable guideline conformance checks under scripts/ci/guidelines/.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GUIDELINES_DIR="${ROOT_DIR}/scripts/ci/guidelines"

echo "::group::Setup"
echo "::notice::Running guideline conformance checks from ${GUIDELINES_DIR}"
echo "::endgroup::"

echo "::group::BuildAndChecks"
if [ ! -d "${GUIDELINES_DIR}" ]; then
  echo "::error::Missing guidelines check directory: ${GUIDELINES_DIR}"
  echo "CI_SIGNAL code=GUIDELINES_FAIL stage=Setup result=failure hint=missing_guidelines_dir"
  exit 1
fi

shopt -s nullglob
checks=("${GUIDELINES_DIR}"/*.sh)
shopt -u nullglob

if [ "${#checks[@]}" -eq 0 ]; then
  echo "::error::No guideline check scripts found in ${GUIDELINES_DIR}"
  echo "CI_SIGNAL code=GUIDELINES_FAIL stage=Setup result=failure hint=no_check_scripts"
  exit 1
fi

for check in "${checks[@]}"; do
  echo "::notice::Running $(basename "${check}")"
  bash "${check}"
done
echo "::endgroup::"

echo "::group::PostRunSummary"
echo "::notice::All guideline checks passed."
echo "::endgroup::"

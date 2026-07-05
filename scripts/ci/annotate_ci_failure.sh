#!/usr/bin/env bash
# Emit GitHub annotations and a step-summary excerpt for the current CI failure.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ARTIFACTS_DIR="${CI_OBSERVABILITY_DIR:-ci-observability}"

fail_code="${CI_FAIL_CODE:--}"
fail_stage="${CI_FAIL_STAGE:--}"
hint="${CI_HINT:--}"
log_override="${1:-}"

pick_log() {
  if [[ -n "${log_override}" && -f "${log_override}" ]]; then
    printf '%s\n' "${log_override}"
    return
  fi

  local candidates=()
  case "${fail_code}" in
    FMT_FAIL) candidates=(fmt.log) ;;
    CLIPPY_FAIL) candidates=(clippy.log) ;;
    TEST_FAIL)
      candidates=(
        test.log
        path-relevance-test.log
        path-filters-sync-test.log
        gate-script-test.log
      )
      ;;
    AUDIT_FAIL) candidates=(audit.log) ;;
    BUILD_FAIL) candidates=(sidecar-build.log) ;;
    ENV_SETUP_FAIL) candidates=(sidecar-pip.log sidecar-proto.log) ;;
    SIDECAR_FAIL) candidates=(stub-sidecar.log rex-agent.log) ;;
    RUFF_FAIL) candidates=(rex-agent.log) ;;
    NPM_CI_FAIL) candidates=(extension-npm-ci.log) ;;
    TYPECHECK_FAIL) candidates=(extension-typecheck.log) ;;
    LINT_FAIL) candidates=(extension-lint.log) ;;
    PACKAGE_FAIL) candidates=(extension-package.log) ;;
    GUIDELINES_FAIL) candidates=(guidelines.log) ;;
    UI_FAIL) candidates=(ui-harness.log ui-web-build.log ui-harness-build.log ui-desktop-build.log) ;;
    *)
      candidates=()
      ;;
  esac

  local f
  for f in "${candidates[@]}"; do
    if [[ -f "${ARTIFACTS_DIR}/${f}" ]] \
      && grep -qE 'FAIL |FAILED|panicked|error|Error:|assertion' "${ARTIFACTS_DIR}/${f}" 2>/dev/null; then
      printf '%s\n' "${ARTIFACTS_DIR}/${f}"
      return
    fi
  done

  if ((${#candidates[@]} > 0)); then
    printf '%s\n' "${ARTIFACTS_DIR}/${candidates[0]}"
    return
  fi

  local newest
  newest="$(find "${ARTIFACTS_DIR}" -maxdepth 1 -type f -name '*.log' 2>/dev/null | head -n 1 || true)"
  if [[ -n "${newest}" ]]; then
    printf '%s\n' "${newest}"
  fi
}

annotate_lines() {
  local excerpt="$1"
  local count=0
  local line clean
  while IFS= read -r line || [[ -n "${line}" ]]; do
    [[ -z "${line}" ]] && continue
    clean="$(printf '%s' "${line}" | sed 's/\x1b\[[0-9;]*m//g')"
    echo "::error title=${fail_code}::${clean}"
    count=$((count + 1))
    if [[ "${count}" -ge 25 ]]; then
      break
    fi
  done <<< "${excerpt}"
}

main() {
  if [[ "${CI_RESULT:-}" == "success" ]]; then
    return 0
  fi

  local log excerpt
  log="$(pick_log || true)"
  if [[ -z "${log}" ]]; then
    echo "::warning::No CI log file found for failure excerpt (code=${fail_code})."
    echo "::error title=${fail_code}::${hint}"
    return 0
  fi

  echo "::group::Failure excerpt (${fail_code} / ${fail_stage})"
  echo "hint: ${hint}"
  echo "log: ${log}"
  excerpt="$("${SCRIPT_DIR}/extract_log_excerpt.sh" "${log}" 40)"
  printf '%s\n' "${excerpt}"
  annotate_lines "${excerpt}"

  if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]] \
    && ! grep -q "### Failure excerpt" "${GITHUB_STEP_SUMMARY}" 2>/dev/null; then
    {
      echo ""
      echo "### Failure excerpt"
      echo ""
      echo "- log: \`${log}\`"
      echo ""
      echo '```text'
      printf '%s\n' "${excerpt}"
      echo '```'
    } >> "${GITHUB_STEP_SUMMARY}"
  fi
  echo "::endgroup::"
}

main "$@"

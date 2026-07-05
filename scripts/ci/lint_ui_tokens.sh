#!/usr/bin/env bash
# Fail when raw color literals appear outside design-system token CSS files.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WEB_SRC="${ROOT}/apps/rex-web/src"
TOKEN_DIR="${WEB_SRC}/design-system/tokens"

fail=0

color_pattern='(#([0-9a-fA-F]{3,8})\b|rgba?\([^)]+\)|hsla?\([^)]+\))'

check_file() {
  local file="$1"
  local rel="${file#"${ROOT}/"}"
  if [[ "${file}" == "${TOKEN_DIR}"/* ]]; then
    return 0
  fi
  if [[ "${file}" == *.css ]] && [[ "${file}" != "${TOKEN_DIR}"/* ]]; then
    while IFS= read -r line; do
      if echo "${line}" | grep -qE "${color_pattern}"; then
        if echo "${line}" | grep -q 'var(--rex-'; then
          continue
        fi
        echo "lint_ui_tokens: raw color in ${rel}:${line}" >&2
        fail=1
      fi
    done < <(grep -nE "${color_pattern}" "${file}" 2>/dev/null || true)
  fi
  if [[ "${file}" == *.tsx ]] || [[ "${file}" == *.ts ]]; then
    if [[ "${file}" == */design-system/theme/* ]]; then
      return 0
    fi
    while IFS= read -r match; do
      echo "lint_ui_tokens: raw color in ${rel}: ${match}" >&2
      fail=1
    done < <(grep -nE "${color_pattern}" "${file}" 2>/dev/null | grep -v 'var(--rex-' || true)
  fi
}

while IFS= read -r -d '' file; do
  check_file "${file}"
done < <(find "${WEB_SRC}" -type f \( -name '*.css' -o -name '*.tsx' -o -name '*.ts' \) ! -path '*/node_modules/*' -print0)

if [ "${fail}" -ne 0 ]; then
  echo "lint_ui_tokens: failed — use --rex-* tokens from design-system/tokens/" >&2
  exit 1
fi

echo "lint_ui_tokens: pass"

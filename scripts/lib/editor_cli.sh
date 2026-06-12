#!/usr/bin/env bash
# Editor CLI helpers for extension install preflight.
# Source from install scripts — do not execute directly.
set -euo pipefail

# Parse VS Code engine version for extension install preflight.
# Sets EDITOR_VSCODE_ENGINE on success (e.g. 1.105.1).
detect_vscode_engine_from_cli() {
  local cli="$1"
  local resolved app_root product_json output line
  EDITOR_VSCODE_ENGINE=""
  if [[ ! -x "${cli}" ]] && ! command -v "${cli}" >/dev/null 2>&1; then
    return 1
  fi
  if command -v "${cli}" >/dev/null 2>&1; then
    cli="$(command -v "${cli}")"
  fi
  if command -v realpath >/dev/null 2>&1; then
    resolved="$(realpath "${cli}" 2>/dev/null || echo "${cli}")"
  else
    resolved="${cli}"
  fi
  # Cursor / VS Code macOS bundles expose vscode engine in Resources/app/product.json.
  if [[ "${resolved}" == *"/Contents/Resources/app/bin/"* ]]; then
    app_root="${resolved%/bin/*}"
    product_json="${app_root}/product.json"
    if [[ -f "${product_json}" ]] && command -v node >/dev/null 2>&1; then
      EDITOR_VSCODE_ENGINE="$(node -p "require('${product_json}').vscodeVersion || ''" 2>/dev/null || true)"
      if [[ -n "${EDITOR_VSCODE_ENGINE}" ]]; then
        return 0
      fi
    fi
  fi
  output="$("${cli}" --version 2>/dev/null || true)"
  if [[ -z "${output}" ]]; then
    return 1
  fi
  # Plain VS Code: first line is the engine version (e.g. 1.96.0).
  while IFS= read -r line; do
    if [[ "${line}" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
      EDITOR_VSCODE_ENGINE="${BASH_REMATCH[1]}.${BASH_REMATCH[2]}.${BASH_REMATCH[3]}"
      return 0
    fi
  done <<<"${output}"
  return 1
}

semver_parse_triple() {
  local version="$1"
  if [[ "${version}" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
    SEMVER_MAJOR="${BASH_REMATCH[1]}"
    SEMVER_MINOR="${BASH_REMATCH[2]}"
    SEMVER_PATCH="${BASH_REMATCH[3]}"
    return 0
  fi
  return 1
}

# Compare host engine against engines.vscode range (supports ^X.Y.Z and >=X.Y.Z).
semver_satisfies_vscode_engine() {
  local host="$1"
  local required_range="$2"
  local req_major req_minor req_patch
  if ! semver_parse_triple "${host}"; then
    return 1
  fi
  local host_major="${SEMVER_MAJOR}"
  local host_minor="${SEMVER_MINOR}"

  if [[ "${required_range}" =~ ^\^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
    req_major="${BASH_REMATCH[1]}"
    req_minor="${BASH_REMATCH[2]}"
    if (( host_major != req_major )); then
      return 1
    fi
    if (( host_minor < req_minor )); then
      return 1
    fi
    return 0
  fi

  if [[ "${required_range}" =~ ^\>=([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
    req_major="${BASH_REMATCH[1]}"
    req_minor="${BASH_REMATCH[2]}"
    req_patch="${BASH_REMATCH[3]}"
    if (( host_major > req_major )); then
      return 0
    fi
    if (( host_major < req_major )); then
      return 1
    fi
    if (( host_minor > req_minor )); then
      return 0
    fi
    if (( host_minor < req_minor )); then
      return 1
    fi
    # Same major.minor — patch on host must be >= required patch (host triple already parsed).
    local host_patch="${SEMVER_PATCH}"
    if (( host_patch >= req_patch )); then
      return 0
    fi
    return 1
  fi

  # Unknown range syntax — do not block install.
  return 0
}

# Preflight extension host against package.json engines.vscode.
# Args: editor_cli path, path to extensions/rex-vscode/package.json
extension_engine_preflight() {
  local editor_cli="$1"
  local package_json="$2"
  local required_range host_engine
  required_range="$(node -p "require('${package_json}').engines.vscode" 2>/dev/null || true)"
  if [[ -z "${required_range}" ]]; then
    echo "WARNING: could not read engines.vscode from ${package_json}" >&2
    return 0
  fi
  if ! detect_vscode_engine_from_cli "${editor_cli}"; then
    cat >&2 <<EOF
Could not detect VS Code engine version from:
  ${editor_cli} --version

Extension requires VS Code engine ${required_range} (see extensions/rex-vscode/package.json).
Upgrade Cursor or VS Code, then re-run ./scripts/install-extension.sh
EOF
    return 1
  fi
  host_engine="${EDITOR_VSCODE_ENGINE}"
  if semver_satisfies_vscode_engine "${host_engine}" "${required_range}"; then
    echo "Editor VS Code engine ${host_engine} satisfies required ${required_range}"
    return 0
  fi
  cat >&2 <<EOF
Extension install blocked: VS Code engine mismatch.

  Host engine:     ${host_engine} (${editor_cli})
  Required range:  ${required_range} (extensions/rex-vscode/package.json)

Next steps:
  - Upgrade Cursor or VS Code to a build that reports engine >= ${required_range#^}
  - Or install a VSIX built for your engine from an older release tag
  - Re-run: ./scripts/install-extension.sh
EOF
  return 1
}

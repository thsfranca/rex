#!/usr/bin/env bash
# Shared helpers for rex-agent Python sidecar install (operator path).
# Source from install scripts — do not execute directly.
set -euo pipefail

REX_AGENT_MIN_PYTHON_MAJOR=3
REX_AGENT_MIN_PYTHON_MINOR=10

python_sidecar__rex_root() {
  if [[ -n "${REX_ROOT:-}" ]]; then
    printf '%s\n' "${REX_ROOT}"
    return 0
  fi
  printf '%s\n' "${HOME}/.rex"
}

python_sidecar__version_ge_310() {
  local major="$1"
  local minor="$2"
  if (( major > REX_AGENT_MIN_PYTHON_MAJOR )); then
    return 0
  fi
  if (( major == REX_AGENT_MIN_PYTHON_MAJOR && minor >= REX_AGENT_MIN_PYTHON_MINOR )); then
    return 0
  fi
  return 1
}

python_sidecar__probe_interpreter() {
  local candidate="$1"
  if ! command -v "${candidate}" >/dev/null 2>&1; then
    return 1
  fi
  local version_line major minor
  version_line="$("${candidate}" --version 2>&1 | head -n1)"
  if [[ "${version_line}" =~ ([0-9]+)\.([0-9]+) ]]; then
    major="${BASH_REMATCH[1]}"
    minor="${BASH_REMATCH[2]}"
    if python_sidecar__version_ge_310 "${major}" "${minor}"; then
      REX_PYTHON="${candidate}"
      REX_PYTHON_VERSION="${major}.${minor}"
      return 0
    fi
  fi
  return 1
}

# Sets REX_PYTHON and REX_PYTHON_VERSION to the best interpreter >= 3.10.
resolve_python_for_rex_agent() {
  REX_PYTHON=""
  REX_PYTHON_VERSION=""
  local candidate
  for candidate in python3.14 python3.13 python3.12 python3.11 python3.10 python3; do
    if python_sidecar__probe_interpreter "${candidate}"; then
      return 0
    fi
  done
  return 1
}

python_sidecar__fail_no_python() {
  local fallback_path=""
  if command -v python3 >/dev/null 2>&1; then
    fallback_path="$(command -v python3) ($("python3" --version 2>&1 | head -n1))"
  fi
  cat >&2 <<EOF
rex-agent requires Python >= ${REX_AGENT_MIN_PYTHON_MAJOR}.${REX_AGENT_MIN_PYTHON_MINOR}.
EOF
  if [[ -n "${fallback_path}" ]]; then
    echo "Found unsupported interpreter: ${fallback_path}" >&2
    echo "macOS Command Line Tools often ship Python 3.9 — do not use it for rex-agent." >&2
  else
    echo "No python3 interpreter found on PATH." >&2
  fi
  cat >&2 <<'EOF'
Next steps:
  - macOS: brew install python@3.12
  - Then re-run: ./scripts/install-agent-sidecar.sh
EOF
  return 1
}

# Creates or reuses $REX_ROOT/venv and upgrades pip tooling inside the venv.
ensure_rex_agent_venv() {
  local rex_root venv_dir
  rex_root="$(python_sidecar__rex_root)"
  venv_dir="${rex_root}/venv"
  REX_VENV_DIR="${venv_dir}"
  REX_VENV_PYTHON="${venv_dir}/bin/python"

  if [[ ! -x "${REX_VENV_PYTHON}" ]]; then
    echo "Creating rex-agent venv at ${venv_dir} (interpreter: ${REX_PYTHON})"
    "${REX_PYTHON}" -m venv "${venv_dir}"
  fi

  echo "Upgrading pip/setuptools/wheel in ${venv_dir}"
  if ! "${REX_VENV_PYTHON}" -m pip install --upgrade pip setuptools wheel >/dev/null; then
    cat >&2 <<EOF
Failed to upgrade pip inside ${venv_dir}.
If you see PEP 668 / externally-managed-environment on system Python, this venv path avoids that —
re-run ./scripts/install-agent-sidecar.sh after installing Python >= 3.10 (brew install python@3.12).
EOF
    return 1
  fi
}

# Installs rex-agent editable from repo root into the venv.
install_rex_agent_editable() {
  local repo_root="$1"
  if [[ -z "${REX_VENV_PYTHON:-}" ]]; then
    echo "install_rex_agent_editable: REX_VENV_PYTHON is unset (call ensure_rex_agent_venv first)" >&2
    return 1
  fi
  "${REX_VENV_PYTHON}" -m pip install -e "${repo_root}/sidecars/rex-agent"
}

# Writes ~/.cargo/bin/rex-agent wrapper that sets PYTHONPATH and execs venv python.
install_rex_agent_wrapper() {
  local rex_root cargo_bin wrapper_path gen_root
  rex_root="$(python_sidecar__rex_root)"
  cargo_bin="${HOME}/.cargo/bin"
  wrapper_path="${cargo_bin}/rex-agent"
  gen_root="${rex_root}/proto/gen"

  if [[ -z "${REX_VENV_PYTHON:-}" ]]; then
    echo "install_rex_agent_wrapper: REX_VENV_PYTHON is unset" >&2
    return 1
  fi

  mkdir -p "${cargo_bin}"
  cat >"${wrapper_path}" <<EOF
#!/usr/bin/env bash
set -euo pipefail
REX_ROOT="\${REX_ROOT:-${rex_root}}"
GEN_ROOT="\${REX_ROOT}/proto/gen"
VENV_PYTHON="${REX_VENV_PYTHON}"
export PYTHONPATH="\${GEN_ROOT}:\${PYTHONPATH:-}"
exec "\${VENV_PYTHON}" -m rex_agent "\$@"
EOF
  chmod +x "${wrapper_path}"
  echo "Installed rex-agent wrapper: ${wrapper_path}"
}

# Full operator install flow. Requires rex on PATH and repo root as $1.
python_sidecar_install() {
  local repo_root="$1"
  if ! resolve_python_for_rex_agent; then
    python_sidecar__fail_no_python
    return 1
  fi
  echo "Using Python ${REX_PYTHON_VERSION} (${REX_PYTHON}) for rex-agent"
  ensure_rex_agent_venv
  install_rex_agent_editable "${repo_root}"
  install_rex_agent_wrapper
}

# Read-only checks for install-preflight.sh — sets REX_PYTHON_STATUS=ok|missing|too_old
python_sidecar_preflight() {
  REX_PYTHON_STATUS="missing"
  REX_PYTHON_DETAIL=""
  if resolve_python_for_rex_agent; then
    REX_PYTHON_STATUS="ok"
    REX_PYTHON_DETAIL="${REX_PYTHON} (${REX_PYTHON_VERSION})"
    return 0
  fi
  if command -v python3 >/dev/null 2>&1; then
    REX_PYTHON_STATUS="too_old"
    REX_PYTHON_DETAIL="$(command -v python3) ($("python3" --version 2>&1 | head -n1))"
    return 1
  fi
  REX_PYTHON_DETAIL="not found"
  return 1
}

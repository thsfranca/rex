#!/usr/bin/env bash
# Maps paths-filter outputs to domain relevance flags for CI workflows.
set -euo pipefail

rust_verify_changed="${RUST_VERIFY_CHANGED:-false}"
sidecar_changed="${SIDECAR_CHANGED:-false}"
guidelines_changed="${GUIDELINES_CHANGED:-false}"
rust_codeql_changed="${RUST_CODEQL_CHANGED:-false}"
python_codeql_changed="${PYTHON_CODEQL_CHANGED:-false}"

rust_relevant=false
sidecar_relevant=false
guidelines_relevant=false
rust_codeql_relevant=false
python_codeql_relevant=false

if [ "${rust_verify_changed}" = "true" ]; then
  rust_relevant=true
fi
if [ "${sidecar_changed}" = "true" ]; then
  sidecar_relevant=true
fi
if [ "${guidelines_changed}" = "true" ]; then
  guidelines_relevant=true
fi

if [ "${rust_codeql_changed}" = "true" ]; then
  rust_codeql_relevant=true
fi
if [ "${python_codeql_changed}" = "true" ]; then
  python_codeql_relevant=true
fi

output_file="${GITHUB_OUTPUT:-/dev/null}"
{
  echo "rust_relevant=${rust_relevant}"
  echo "sidecar_relevant=${sidecar_relevant}"
  echo "guidelines_relevant=${guidelines_relevant}"
  echo "rust_codeql_relevant=${rust_codeql_relevant}"
  echo "python_codeql_relevant=${python_codeql_relevant}"
} >> "${output_file}"

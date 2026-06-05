#!/usr/bin/env bash
# Maps paths-filter outputs to domain relevance flags for CI workflows.
set -euo pipefail

rust_changed="${RUST_CHANGED:-false}"
extension_changed="${EXTENSION_CHANGED:-false}"
sidecar_changed="${SIDECAR_CHANGED:-false}"
guidelines_changed="${GUIDELINES_CHANGED:-false}"
rust_codeql_changed="${RUST_CODEQL_CHANGED:-false}"
extension_codeql_changed="${EXTENSION_CODEQL_CHANGED:-false}"
python_codeql_changed="${PYTHON_CODEQL_CHANGED:-false}"
ci_changed="${CI_CHANGED:-false}"
global_changed="${GLOBAL_CHANGED:-false}"

rust_relevant=false
extension_relevant=false
sidecar_relevant=false
guidelines_relevant=false
rust_codeql_relevant=false
extension_codeql_relevant=false
python_codeql_relevant=false

if [ "${rust_changed}" = "true" ] || [ "${ci_changed}" = "true" ] || [ "${global_changed}" = "true" ]; then
  rust_relevant=true
fi
if [ "${extension_changed}" = "true" ] || [ "${ci_changed}" = "true" ] || [ "${global_changed}" = "true" ]; then
  extension_relevant=true
fi
if [ "${sidecar_changed}" = "true" ]; then
  sidecar_relevant=true
fi
if [ "${guidelines_changed}" = "true" ]; then
  guidelines_relevant=true
fi

# CodeQL uses source-only path gates on PR/push; weekly schedule bypasses these in codeql.yml.
if [ "${rust_codeql_changed}" = "true" ]; then
  rust_codeql_relevant=true
fi
if [ "${extension_codeql_changed}" = "true" ]; then
  extension_codeql_relevant=true
fi
if [ "${python_codeql_changed}" = "true" ]; then
  python_codeql_relevant=true
fi

output_file="${GITHUB_OUTPUT:-/dev/null}"
{
  echo "rust_relevant=${rust_relevant}"
  echo "extension_relevant=${extension_relevant}"
  echo "sidecar_relevant=${sidecar_relevant}"
  echo "guidelines_relevant=${guidelines_relevant}"
  echo "rust_codeql_relevant=${rust_codeql_relevant}"
  echo "extension_codeql_relevant=${extension_codeql_relevant}"
  echo "python_codeql_relevant=${python_codeql_relevant}"
} >> "${output_file}"

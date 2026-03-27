#!/usr/bin/env bash
set -euo pipefail

for result in "$@"; do
  if [[ "$result" == "failure" || "$result" == "cancelled" ]]; then
    exit 1
  fi
done

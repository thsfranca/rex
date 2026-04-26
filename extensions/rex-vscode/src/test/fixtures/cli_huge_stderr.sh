#!/usr/bin/env bash
# Floods stderr; exits non-zero without a JSON terminal line (exercise stderr cap on close).
set -u
for i in $(seq 1 4000); do
  echo "repeated-stderr-explanation=$i" >&2
done
exit 2

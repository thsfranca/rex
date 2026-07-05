#!/usr/bin/env bash
# Validates docs/manifest.yaml against active docs in docs/README.md repository map.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
MANIFEST="${ROOT_DIR}/docs/manifest.yaml"

if [ ! -f "${MANIFEST}" ]; then
  echo "::error::Missing docs/manifest.yaml"
  exit 1
fi

echo "::group::Docs manifest catalog"
python3 - "${MANIFEST}" "${ROOT_DIR}" <<'PY'
import re
import sys
from pathlib import Path

manifest_path = Path(sys.argv[1])
root = Path(sys.argv[2])

text = manifest_path.read_text()
paths = re.findall(r"^\s+- path:\s+(\S+)\s*$", text, re.MULTILINE)
if not paths:
    print("::error::manifest has no document paths")
    sys.exit(1)

seen = set()
for p in paths:
    if p in seen:
        print(f"::error::duplicate manifest path: {p}")
        sys.exit(1)
    seen.add(p)
    full = root / p
    if not full.is_file():
        print(f"::error::manifest path missing on disk: {p}")
        sys.exit(1)
    if p.startswith("docs/historical/"):
        print(f"::error::cancelled doc in active manifest: {p}")
        sys.exit(1)

required = [
    "docs/AGENTS.md",
    "docs/ERROR_HANDLING.md",
    "docs/STREAM_EVENTS.md",
    "docs/CONFIGURATION.md",
]
for req in required:
    if req not in seen:
        print(f"::error::required manifest entry missing: {req}")
        sys.exit(1)

if "docs/ROADMAP.md" not in seen:
    print("::error::manifest should include docs/ROADMAP.md")
    sys.exit(1)

print(f"::notice::Validated {len(paths)} manifest entries")
PY
echo "::endgroup::"

echo "::notice::Docs manifest checks passed."

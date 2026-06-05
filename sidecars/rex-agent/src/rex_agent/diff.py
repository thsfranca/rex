"""Unified diff application for Editor writes (R030).

Sidecar-local patch-before-write.
"""

from __future__ import annotations

import re
from typing import Any

WHOLE_FILE_LINE_LIMIT = 50
MIN_CONTEXT_LINES = 3


def editor_write_prompt_suffix() -> str:
    return (
        "For fs.write on files over 50 lines, use unified diff format only:\n"
        '{"type":"tool","tool":"fs.write","args":{"path":"...","diff":"..."}}\n'
        f"Diff must include at least {MIN_CONTEXT_LINES} context lines per hunk."
    )


def reject_whole_file_write(path: str, content: str, existing: str) -> str | None:
    line_count = max(len(existing.splitlines()), len(content.splitlines()), 1)
    if (
        line_count > WHOLE_FILE_LINE_LIMIT
        and content
        and not content.lstrip().startswith("---")
    ):
        return (
            f"Whole-file write rejected for {path!r} ({line_count} lines). "
            "Use args.diff with a unified diff (≥3 context lines)."
        )
    return None


def apply_unified_diff(original: str, diff_text: str) -> tuple[bool, str]:
    if not diff_text.strip():
        return False, "Patch failed: empty diff"
    result = _apply_unified_diff_manual(original, diff_text)
    if result is None:
        return False, "Patch failed: could not apply unified diff."
    return True, result


def _apply_unified_diff_manual(original: str, diff_text: str) -> str | None:
    orig_lines = original.splitlines(keepends=True)
    if original and not original.endswith("\n") and orig_lines:
        orig_lines[-1] = orig_lines[-1].rstrip("\n") + "\n"

    hunks = _parse_hunks(diff_text.splitlines())
    if not hunks:
        return None

    result: list[str] = []
    orig_idx = 0
    for hunk in hunks:
        start = hunk["old_start"]
        while orig_idx < start - 1 and orig_idx < len(orig_lines):
            result.append(orig_lines[orig_idx])
            orig_idx += 1
        for line in hunk["lines"]:
            tag = line[0]
            text = line[1:]
            if tag == " ":
                if orig_idx < len(orig_lines):
                    result.append(orig_lines[orig_idx])
                    orig_idx += 1
            elif tag == "-":
                if orig_idx < len(orig_lines):
                    orig_idx += 1
            elif tag == "+":
                result.append(text if text.endswith("\n") else text + "\n")
    while orig_idx < len(orig_lines):
        result.append(orig_lines[orig_idx])
        orig_idx += 1
    return "".join(result)


def _parse_hunks(diff_lines: list[str]) -> list[dict[str, Any]]:
    hunks: list[dict[str, Any]] = []
    i = 0
    while i < len(diff_lines):
        line = diff_lines[i]
        match = re.match(r"^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@", line)
        if not match:
            i += 1
            continue
        hunk = {"old_start": int(match.group(1)), "lines": []}
        i += 1
        while i < len(diff_lines) and not diff_lines[i].startswith("@@"):
            if diff_lines[i] and diff_lines[i][0] in (" ", "+", "-"):
                hunk["lines"].append(diff_lines[i])
            i += 1
        hunks.append(hunk)
    return hunks

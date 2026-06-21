"""Advisory prompt intent detection (R067 / R070)."""

from __future__ import annotations

ADVISORY_PATTERNS: tuple[str, ...] = (
    "what should we do next",
    "what's next",
    "what to work on",
    "priorities",
    "roadmap",
    "next step",
)

PRIORITY_CONTEXT_MARKERS: tuple[str, ...] = (
    "roadmap",
    "prioritization",
    "current focus",
)


def matches_advisory_intent(text: str) -> bool:
    lower = (text or "").lower()
    return any(pattern in lower for pattern in ADVISORY_PATTERNS)


def daemon_context_has_priority_markers(text: str) -> bool:
    lower = (text or "").lower()
    return any(marker in lower for marker in PRIORITY_CONTEXT_MARKERS)

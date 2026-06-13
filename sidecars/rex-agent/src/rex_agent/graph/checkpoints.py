"""In-memory soft-cap turn checkpoints for ContinueTurn (R063)."""

from __future__ import annotations

import copy
import secrets
from typing import Any

from rex_agent.graph.state import AgentState

_store: dict[str, AgentState] = {}


def save_soft_cap_checkpoint(state: AgentState) -> str:
    token = secrets.token_urlsafe(16)
    _store[token] = copy.deepcopy(state)
    return token


def load_soft_cap_checkpoint(token: str) -> AgentState | None:
    trimmed = (token or "").strip()
    if not trimmed:
        return None
    state = _store.get(trimmed)
    if state is None:
        return None
    return copy.deepcopy(state)


def consume_soft_cap_checkpoint(token: str) -> AgentState | None:
    trimmed = (token or "").strip()
    if not trimmed:
        return None
    state = _store.pop(trimmed, None)
    if state is None:
        return None
    return copy.deepcopy(state)


def clear_checkpoints_for_tests() -> None:
    _store.clear()

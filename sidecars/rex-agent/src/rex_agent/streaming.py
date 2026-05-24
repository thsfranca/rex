"""Chunk RunTurn text for incremental streaming."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class TextChunk:
    text: str
    index: int
    done: bool


def chunk_text(text: str, max_chars: int = 8) -> list[str]:
    if not text:
        return []
    size = max(1, max_chars)
    chars = list(text)
    return ["".join(chars[i : i + size]) for i in range(0, len(chars), size)]


def run_turn_chunks(text: str, max_chars: int = 8) -> list[TextChunk]:
    """Build content chunks plus a terminal done chunk (stub index semantics)."""
    pieces = chunk_text(text, max_chars)
    chunks: list[TextChunk] = [
        TextChunk(text=piece, index=index, done=False)
        for index, piece in enumerate(pieces)
    ]
    terminal_index = len(pieces)
    chunks.append(TextChunk(text="", index=terminal_index, done=True))
    return chunks

from __future__ import annotations

from collections.abc import AsyncIterator


async def stream_completion(response) -> AsyncIterator[str]:
    async for chunk in response:
        yield f"data: {chunk.model_dump_json()}\n\n"
    yield "data: [DONE]\n\n"

from __future__ import annotations

import logging
import time
from collections.abc import AsyncIterator

logger = logging.getLogger(__name__)


async def stream_completion(response, timeout: float = 600) -> AsyncIterator[str]:
    deadline = time.perf_counter() + timeout
    async for chunk in response:
        if time.perf_counter() > deadline:
            logger.warning("Stream wall-clock limit reached (%.1fs)", timeout)
            break
        yield f"data: {chunk.model_dump_json()}\n\n"
    yield "data: [DONE]\n\n"

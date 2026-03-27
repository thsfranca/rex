from __future__ import annotations

import pytest

from app.proxy.streaming import stream_completion


class FakeChunk:
    def __init__(self, data: str):
        self._data = data

    def model_dump_json(self) -> str:
        return self._data


async def _async_iter(items):
    for item in items:
        yield item


@pytest.mark.asyncio
async def test_stream_yields_data_lines():
    chunks = [FakeChunk('{"id":"1"}'), FakeChunk('{"id":"2"}')]
    results = [line async for line in stream_completion(_async_iter(chunks))]

    assert results[0] == 'data: {"id":"1"}\n\n'
    assert results[1] == 'data: {"id":"2"}\n\n'


@pytest.mark.asyncio
async def test_stream_ends_with_done():
    chunks = [FakeChunk('{"id":"1"}')]
    results = [line async for line in stream_completion(_async_iter(chunks))]

    assert results[-1] == "data: [DONE]\n\n"


@pytest.mark.asyncio
async def test_stream_empty_response():
    results = [line async for line in stream_completion(_async_iter([]))]

    assert results == ["data: [DONE]\n\n"]

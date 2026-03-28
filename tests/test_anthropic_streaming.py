from __future__ import annotations

import json
from unittest.mock import AsyncMock, MagicMock, patch

import pytest
from fastapi.responses import StreamingResponse

from app.config import ModelConfig
from app.proxy.anthropic import stream_anthropic_response
from app.proxy.handler import handle_anthropic_messages
from app.router.engine import RoutingEngine
from app.router.registry import ModelRegistry


def _make_model(**overrides) -> ModelConfig:
    defaults = {"name": "test/model"}
    defaults.update(overrides)
    return ModelConfig(**defaults)


def _make_engine(models: list[ModelConfig]) -> RoutingEngine:
    registry = ModelRegistry(models)
    return RoutingEngine(registry)


def _make_request(api_key: str | None = None) -> MagicMock:
    request = MagicMock()
    headers = {}
    if api_key:
        headers["x-api-key"] = api_key
    request.headers = headers
    return request


class FakeStreamChunk:
    def __init__(self, content=None, finish_reason=None, tool_calls=None):
        choice = MagicMock()
        choice.delta = MagicMock()
        choice.delta.content = content
        choice.delta.tool_calls = tool_calls
        choice.finish_reason = finish_reason
        self.choices = [choice]
        self.usage = None


class FakeToolCallDelta:
    def __init__(self, index=0, tc_id=None, name=None, arguments=None):
        self.index = index
        self.id = tc_id
        self.function = MagicMock()
        self.function.name = name
        self.function.arguments = arguments


async def _async_iter(items):
    for item in items:
        yield item


def _parse_sse_events(raw_events: list[str]) -> list[tuple[str, dict]]:
    parsed = []
    for raw in raw_events:
        lines = raw.strip().split("\n")
        event_type = None
        data = None
        for line in lines:
            if line.startswith("event: "):
                event_type = line[7:]
            elif line.startswith("data: "):
                data = json.loads(line[6:])
        if event_type and data:
            parsed.append((event_type, data))
    return parsed


class TestStreamAnthropicResponse:
    @pytest.mark.asyncio
    async def test_emits_message_start(self):
        chunks = [FakeStreamChunk(content="Hi")]
        events = [e async for e in stream_anthropic_response(_async_iter(chunks), "test/model")]
        parsed = _parse_sse_events(events)

        event_type, data = parsed[0]
        assert event_type == "message_start"
        assert data["type"] == "message_start"
        assert data["message"]["type"] == "message"
        assert data["message"]["role"] == "assistant"

    @pytest.mark.asyncio
    async def test_emits_content_block_start(self):
        chunks = [FakeStreamChunk(content="Hi")]
        events = [e async for e in stream_anthropic_response(_async_iter(chunks), "test/model")]
        parsed = _parse_sse_events(events)

        event_type, data = parsed[1]
        assert event_type == "content_block_start"
        assert data["content_block"]["type"] == "text"

    @pytest.mark.asyncio
    async def test_emits_text_deltas(self):
        chunks = [
            FakeStreamChunk(content="Hello"),
            FakeStreamChunk(content=" world"),
        ]
        events = [e async for e in stream_anthropic_response(_async_iter(chunks), "test/model")]
        parsed = _parse_sse_events(events)

        deltas = [(t, d) for t, d in parsed if t == "content_block_delta"]
        assert len(deltas) == 2
        assert deltas[0][1]["delta"]["text"] == "Hello"
        assert deltas[1][1]["delta"]["text"] == " world"

    @pytest.mark.asyncio
    async def test_emits_content_block_stop(self):
        chunks = [FakeStreamChunk(content="Hi")]
        events = [e async for e in stream_anthropic_response(_async_iter(chunks), "test/model")]
        parsed = _parse_sse_events(events)

        stop_events = [(t, d) for t, d in parsed if t == "content_block_stop"]
        assert len(stop_events) == 1

    @pytest.mark.asyncio
    async def test_emits_message_delta_with_stop_reason(self):
        chunks = [FakeStreamChunk(content="Hi", finish_reason="stop")]
        events = [e async for e in stream_anthropic_response(_async_iter(chunks), "test/model")]
        parsed = _parse_sse_events(events)

        delta_events = [(t, d) for t, d in parsed if t == "message_delta"]
        assert len(delta_events) == 1
        assert delta_events[0][1]["delta"]["stop_reason"] == "end_turn"

    @pytest.mark.asyncio
    async def test_maps_length_to_max_tokens(self):
        chunks = [FakeStreamChunk(content="Hi", finish_reason="length")]
        events = [e async for e in stream_anthropic_response(_async_iter(chunks), "test/model")]
        parsed = _parse_sse_events(events)

        delta_events = [(t, d) for t, d in parsed if t == "message_delta"]
        assert delta_events[0][1]["delta"]["stop_reason"] == "max_tokens"

    @pytest.mark.asyncio
    async def test_emits_message_stop(self):
        chunks = [FakeStreamChunk(content="Hi")]
        events = [e async for e in stream_anthropic_response(_async_iter(chunks), "test/model")]
        parsed = _parse_sse_events(events)

        stop_events = [(t, d) for t, d in parsed if t == "message_stop"]
        assert len(stop_events) == 1

    @pytest.mark.asyncio
    async def test_echoes_request_model(self):
        chunks = [FakeStreamChunk(content="Hi")]
        events = [
            e
            async for e in stream_anthropic_response(
                _async_iter(chunks), "ollama/llama3", "claude-3-sonnet"
            )
        ]
        parsed = _parse_sse_events(events)

        _, data = parsed[0]
        assert data["message"]["model"] == "claude-3-sonnet"

    @pytest.mark.asyncio
    async def test_event_sequence_order(self):
        chunks = [FakeStreamChunk(content="Hi", finish_reason="stop")]
        events = [e async for e in stream_anthropic_response(_async_iter(chunks), "test/model")]
        parsed = _parse_sse_events(events)

        event_types = [t for t, _ in parsed]
        assert event_types[0] == "message_start"
        assert event_types[1] == "content_block_start"
        assert event_types[2] == "ping"
        assert event_types[-3] == "content_block_stop"
        assert event_types[-2] == "message_delta"
        assert event_types[-1] == "message_stop"

    @pytest.mark.asyncio
    async def test_empty_stream(self):
        events = [e async for e in stream_anthropic_response(_async_iter([]), "test/model")]
        parsed = _parse_sse_events(events)

        event_types = [t for t, _ in parsed]
        assert "message_start" in event_types
        assert "content_block_start" in event_types
        assert "content_block_stop" in event_types
        assert "message_delta" in event_types
        assert "message_stop" in event_types

    @pytest.mark.asyncio
    async def test_skips_chunks_without_content(self):
        chunks = [
            FakeStreamChunk(content=None),
            FakeStreamChunk(content="Hello"),
            FakeStreamChunk(content=None, finish_reason="stop"),
        ]
        events = [e async for e in stream_anthropic_response(_async_iter(chunks), "test/model")]
        parsed = _parse_sse_events(events)

        deltas = [(t, d) for t, d in parsed if t == "content_block_delta"]
        assert len(deltas) == 1
        assert deltas[0][1]["delta"]["text"] == "Hello"


class TestStreamAnthropicToolUse:
    @pytest.mark.asyncio
    async def test_tool_call_emits_tool_use_block(self):
        chunks = [
            FakeStreamChunk(
                tool_calls=[FakeToolCallDelta(index=0, tc_id="call_1", name="bash", arguments="")],
            ),
            FakeStreamChunk(
                tool_calls=[FakeToolCallDelta(index=0, arguments='{"command": "ls"}')],
            ),
            FakeStreamChunk(finish_reason="tool_calls"),
        ]
        events = [e async for e in stream_anthropic_response(_async_iter(chunks), "test/model")]
        parsed = _parse_sse_events(events)

        block_starts = [(t, d) for t, d in parsed if t == "content_block_start"]
        assert len(block_starts) == 2
        assert block_starts[0][1]["content_block"]["type"] == "text"
        assert block_starts[1][1]["content_block"]["type"] == "tool_use"
        assert block_starts[1][1]["content_block"]["id"] == "call_1"
        assert block_starts[1][1]["content_block"]["name"] == "bash"

    @pytest.mark.asyncio
    async def test_tool_call_emits_input_json_delta(self):
        chunks = [
            FakeStreamChunk(
                tool_calls=[FakeToolCallDelta(index=0, tc_id="call_1", name="bash", arguments="")],
            ),
            FakeStreamChunk(
                tool_calls=[FakeToolCallDelta(index=0, arguments='{"command":')],
            ),
            FakeStreamChunk(
                tool_calls=[FakeToolCallDelta(index=0, arguments=' "ls"}')],
            ),
            FakeStreamChunk(finish_reason="tool_calls"),
        ]
        events = [e async for e in stream_anthropic_response(_async_iter(chunks), "test/model")]
        parsed = _parse_sse_events(events)

        json_deltas = [
            d
            for t, d in parsed
            if t == "content_block_delta" and d["delta"]["type"] == "input_json_delta"
        ]
        assert len(json_deltas) == 2
        assert json_deltas[0]["delta"]["partial_json"] == '{"command":'
        assert json_deltas[1]["delta"]["partial_json"] == ' "ls"}'

    @pytest.mark.asyncio
    async def test_tool_call_stop_reason_is_tool_use(self):
        chunks = [
            FakeStreamChunk(
                tool_calls=[
                    FakeToolCallDelta(index=0, tc_id="call_1", name="bash", arguments="{}")
                ],
            ),
            FakeStreamChunk(finish_reason="tool_calls"),
        ]
        events = [e async for e in stream_anthropic_response(_async_iter(chunks), "test/model")]
        parsed = _parse_sse_events(events)

        delta_events = [(t, d) for t, d in parsed if t == "message_delta"]
        assert delta_events[0][1]["delta"]["stop_reason"] == "tool_use"

    @pytest.mark.asyncio
    async def test_text_then_tool_call(self):
        chunks = [
            FakeStreamChunk(content="I'll run ls."),
            FakeStreamChunk(
                tool_calls=[
                    FakeToolCallDelta(index=0, tc_id="call_1", name="bash", arguments="{}")
                ],
            ),
            FakeStreamChunk(finish_reason="tool_calls"),
        ]
        events = [e async for e in stream_anthropic_response(_async_iter(chunks), "test/model")]
        parsed = _parse_sse_events(events)

        text_deltas = [
            d
            for t, d in parsed
            if t == "content_block_delta" and d["delta"]["type"] == "text_delta"
        ]
        assert len(text_deltas) == 1
        assert text_deltas[0]["delta"]["text"] == "I'll run ls."

        tool_starts = [
            d
            for t, d in parsed
            if t == "content_block_start" and d["content_block"]["type"] == "tool_use"
        ]
        assert len(tool_starts) == 1

    @pytest.mark.asyncio
    async def test_multiple_tool_calls(self):
        chunks = [
            FakeStreamChunk(
                tool_calls=[
                    FakeToolCallDelta(index=0, tc_id="call_1", name="bash", arguments="{}")
                ],
            ),
            FakeStreamChunk(
                tool_calls=[
                    FakeToolCallDelta(index=1, tc_id="call_2", name="read", arguments="{}")
                ],
            ),
            FakeStreamChunk(finish_reason="tool_calls"),
        ]
        events = [e async for e in stream_anthropic_response(_async_iter(chunks), "test/model")]
        parsed = _parse_sse_events(events)

        tool_starts = [
            d
            for t, d in parsed
            if t == "content_block_start" and d["content_block"]["type"] == "tool_use"
        ]
        assert len(tool_starts) == 2
        assert tool_starts[0]["content_block"]["name"] == "bash"
        assert tool_starts[1]["content_block"]["name"] == "read"

    @pytest.mark.asyncio
    async def test_tool_call_blocks_are_closed(self):
        chunks = [
            FakeStreamChunk(
                tool_calls=[
                    FakeToolCallDelta(index=0, tc_id="call_1", name="bash", arguments="{}")
                ],
            ),
            FakeStreamChunk(finish_reason="tool_calls"),
        ]
        events = [e async for e in stream_anthropic_response(_async_iter(chunks), "test/model")]
        parsed = _parse_sse_events(events)

        block_stops = [(t, d) for t, d in parsed if t == "content_block_stop"]
        assert len(block_stops) == 2


class TestHandleAnthropicMessagesStreaming:
    @pytest.mark.asyncio
    @patch("app.proxy.handler.litellm")
    async def test_streaming_returns_sse_response(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(
            return_value=_async_iter([FakeStreamChunk(content="Hi")])
        )
        model = _make_model(name="test/model")
        engine = _make_engine([model])
        request = _make_request()

        body = {
            "model": "claude-3-sonnet",
            "max_tokens": 1024,
            "stream": True,
            "messages": [{"role": "user", "content": "Hello"}],
        }
        response = await handle_anthropic_messages(body, engine, request)

        assert isinstance(response, StreamingResponse)
        assert response.media_type == "text/event-stream"

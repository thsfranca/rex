from __future__ import annotations

from app.adapters.base import ClientAdapter, NormalizedRequest
from app.adapters.default import DefaultAdapter
from app.adapters.registry import AdapterRegistry
from app.router.detector import FeatureType


class TestNormalizedRequest:
    def test_is_frozen(self):
        req = NormalizedRequest(
            messages=[{"role": "user", "content": "hello"}],
            feature_type=FeatureType.CHAT,
        )
        try:
            req.feature_type = FeatureType.COMPLETION
            assert False, "Should have raised"
        except AttributeError:
            pass

    def test_defaults(self):
        req = NormalizedRequest(
            messages=[],
            feature_type=FeatureType.CHAT,
        )
        assert req.max_tokens is None
        assert req.temperature is None


class TestDefaultAdapter:
    def test_detects_completion(self):
        adapter = DefaultAdapter()
        result = adapter.normalize(
            messages=[{"role": "user", "content": "x"}],
            max_tokens=50,
            temperature=0.0,
        )
        assert result.feature_type == FeatureType.COMPLETION
        assert result.messages == [{"role": "user", "content": "x"}]
        assert result.max_tokens == 50
        assert result.temperature == 0.0

    def test_detects_chat(self):
        adapter = DefaultAdapter()
        result = adapter.normalize(
            messages=[
                {"role": "user", "content": "Explain async in Python"},
                {"role": "assistant", "content": "..."},
                {"role": "user", "content": "Show me an example"},
            ],
        )
        assert result.feature_type == FeatureType.CHAT

    def test_passes_through_messages(self):
        adapter = DefaultAdapter()
        messages = [{"role": "user", "content": "hello"}]
        result = adapter.normalize(messages=messages)
        assert result.messages is messages

    def test_is_client_adapter(self):
        adapter = DefaultAdapter()
        assert isinstance(adapter, ClientAdapter)


class FakeAdapter(ClientAdapter):
    def __init__(self, feature_type: FeatureType = FeatureType.CHAT):
        self._feature_type = feature_type

    def normalize(
        self,
        messages: list[dict],
        max_tokens: int | None = None,
        temperature: float | None = None,
    ) -> NormalizedRequest:
        return NormalizedRequest(
            messages=messages,
            feature_type=self._feature_type,
            max_tokens=max_tokens,
            temperature=temperature,
        )


class TestAdapterRegistry:
    def test_returns_default_for_no_user_agent(self):
        registry = AdapterRegistry()
        adapter = registry.get_adapter(None)
        assert isinstance(adapter, DefaultAdapter)

    def test_returns_default_for_unknown_user_agent(self):
        registry = AdapterRegistry()
        adapter = registry.get_adapter("SomeUnknownTool/1.0")
        assert isinstance(adapter, DefaultAdapter)

    def test_returns_registered_adapter(self):
        registry = AdapterRegistry()
        fake = FakeAdapter()
        registry.register("cursor", fake)
        adapter = registry.get_adapter("Cursor/0.50.0")
        assert adapter is fake

    def test_matching_is_case_insensitive(self):
        registry = AdapterRegistry()
        fake = FakeAdapter()
        registry.register("CURSOR", fake)
        adapter = registry.get_adapter("cursor/0.50.0")
        assert adapter is fake

    def test_multiple_adapters(self):
        registry = AdapterRegistry()
        cursor_adapter = FakeAdapter(FeatureType.COMPLETION)
        claude_adapter = FakeAdapter(FeatureType.CHAT)
        registry.register("cursor", cursor_adapter)
        registry.register("claude-code", claude_adapter)

        assert registry.get_adapter("Cursor/1.0") is cursor_adapter
        assert registry.get_adapter("Claude-Code/0.1") is claude_adapter
        assert isinstance(registry.get_adapter("Unknown/1.0"), DefaultAdapter)

    def test_empty_user_agent(self):
        registry = AdapterRegistry()
        adapter = registry.get_adapter("")
        assert isinstance(adapter, DefaultAdapter)

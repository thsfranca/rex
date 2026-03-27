from __future__ import annotations

from app.router.detector import FeatureType, detect_feature


class TestDetectFeature:
    def test_short_single_turn_low_temp_is_completion(self):
        messages = [{"role": "user", "content": "def hello"}]
        result = detect_feature(messages, max_tokens=50, temperature=0)
        assert result == FeatureType.COMPLETION

    def test_multi_turn_is_chat(self):
        messages = [
            {"role": "system", "content": "You are a helpful assistant"},
            {"role": "user", "content": "Explain async in Python"},
            {"role": "assistant", "content": "Async is..."},
            {"role": "user", "content": "Show me an example"},
        ]
        result = detect_feature(messages)
        assert result == FeatureType.CHAT

    def test_long_prompt_single_turn_defaults_to_chat(self):
        messages = [{"role": "user", "content": "x" * 600}]
        result = detect_feature(messages)
        assert result == FeatureType.CHAT

    def test_single_turn_no_params_defaults_to_chat(self):
        messages = [{"role": "user", "content": "Explain how decorators work in Python"}]
        result = detect_feature(messages)
        assert result == FeatureType.CHAT

    def test_single_turn_low_max_tokens_low_temp_is_completion(self):
        messages = [{"role": "user", "content": "complete this"}]
        result = detect_feature(messages, max_tokens=100, temperature=0.1)
        assert result == FeatureType.COMPLETION

    def test_high_temperature_pushes_toward_chat(self):
        messages = [{"role": "user", "content": "short"}]
        result = detect_feature(messages, temperature=0.9)
        assert result == FeatureType.CHAT

    def test_empty_messages_defaults_to_chat(self):
        result = detect_feature([])
        assert result == FeatureType.CHAT

    def test_system_only_messages(self):
        messages = [{"role": "system", "content": "You are a code assistant"}]
        result = detect_feature(messages)
        assert result == FeatureType.CHAT

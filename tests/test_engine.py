from __future__ import annotations

import json
from unittest.mock import AsyncMock, MagicMock, patch

import numpy as np
import pytest

from app.config import ModelConfig
from app.learning.centroids import CentroidClassifier
from app.router.categories import TaskCategory
from app.router.classifier import ClassificationResult
from app.router.detector import FeatureType
from app.router.engine import RoutingDecision, RoutingEngine
from app.router.llm_judge import LLMJudge
from app.router.registry import ModelRegistry


def _make_model(**overrides) -> ModelConfig:
    defaults = {"name": "test/model"}
    defaults.update(overrides)
    return ModelConfig(**defaults)


def _make_engine(
    models: list[ModelConfig],
    primary_model: str | None = None,
    judge: LLMJudge | None = None,
    confidence_threshold: float = 0.5,
    centroid_classifier: CentroidClassifier | None = None,
) -> RoutingEngine:
    registry = ModelRegistry(models)
    return RoutingEngine(registry, primary_model, judge, confidence_threshold, centroid_classifier)


class TestSelectModel:
    @pytest.mark.asyncio
    async def test_routes_to_primary_model(self):
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001)
        expensive = _make_model(name="expensive/model", cost_per_1k_input=0.03)
        engine = _make_engine([cheap, expensive], confidence_threshold=0.0)

        messages = [{"role": "user", "content": "hello"}]
        decision = await engine.select_model(messages)
        assert decision.model.name == "cheap/model"

    @pytest.mark.asyncio
    async def test_routes_to_explicit_primary(self):
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001)
        expensive = _make_model(name="expensive/model", cost_per_1k_input=0.03)
        engine = _make_engine([cheap, expensive], primary_model="expensive/model")

        messages = [{"role": "user", "content": "hello"}]
        decision = await engine.select_model(messages)
        assert decision.model.name == "expensive/model"

    @pytest.mark.asyncio
    async def test_auto_selects_local_over_cloud(self):
        cloud = _make_model(name="cloud/model", cost_per_1k_input=0.001)
        local = _make_model(name="local/model", is_local=True, cost_per_1k_input=0.0)
        engine = _make_engine([cloud, local], confidence_threshold=0.0)

        messages = [{"role": "user", "content": "hello"}]
        decision = await engine.select_model(messages)
        assert decision.model.name == "local/model"

    @pytest.mark.asyncio
    async def test_single_model(self):
        model = _make_model(name="only/model")
        engine = _make_engine([model])

        messages = [{"role": "user", "content": "hello"}]
        decision = await engine.select_model(messages)
        assert decision.model.name == "only/model"

    @pytest.mark.asyncio
    async def test_uses_provided_feature_type(self):
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001)
        expensive = _make_model(name="expensive/model", cost_per_1k_input=0.03)
        engine = _make_engine([cheap, expensive])

        messages = [
            {"role": "user", "content": "Explain async in Python"},
            {"role": "assistant", "content": "..."},
            {"role": "user", "content": "Show me an example"},
        ]
        decision = await engine.select_model(messages, feature_type=FeatureType.COMPLETION)
        assert decision.model.name == "cheap/model"

    @pytest.mark.asyncio
    async def test_feature_type_none_falls_back_to_detection(self):
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001)
        engine = _make_engine([cheap])

        messages = [
            {"role": "user", "content": "Explain async"},
            {"role": "assistant", "content": "..."},
            {"role": "user", "content": "More details"},
        ]
        decision = await engine.select_model(messages, feature_type=None)
        assert decision.model.name == "cheap/model"

    @pytest.mark.asyncio
    async def test_returns_routing_decision(self):
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001)
        engine = _make_engine([cheap])

        messages = [{"role": "user", "content": "hello"}]
        decision = await engine.select_model(messages)
        assert isinstance(decision, RoutingDecision)
        assert decision.model.name == "cheap/model"
        assert isinstance(decision.category, TaskCategory)
        assert isinstance(decision.confidence, float)
        assert isinstance(decision.feature_type, FeatureType)


class TestTaskAwareRouting:
    @pytest.mark.asyncio
    async def test_completion_always_routes_to_primary(self):
        cheap = _make_model(
            name="cheap/model",
            cost_per_1k_input=0.001,
            max_context_window=8000,
        )
        large = _make_model(
            name="large/model",
            cost_per_1k_input=0.03,
            max_context_window=128000,
        )
        engine = _make_engine([cheap, large])

        messages = [{"role": "user", "content": "x"}]
        decision = await engine.select_model(messages, max_tokens=100, temperature=0.0)
        assert decision.model.name == "cheap/model"
        assert decision.category == TaskCategory.COMPLETION
        assert decision.feature_type == FeatureType.COMPLETION

    @pytest.mark.asyncio
    async def test_refactoring_upgrades_when_primary_too_small(self):
        small = _make_model(
            name="small/model",
            cost_per_1k_input=0.001,
            max_context_window=8000,
        )
        large = _make_model(
            name="large/model",
            cost_per_1k_input=0.01,
            max_context_window=128000,
        )
        engine = _make_engine([small, large])

        messages = [
            {"role": "user", "content": "Please refactor this entire module"},
        ]
        decision = await engine.select_model(messages)
        assert decision.model.name == "large/model"
        assert decision.category == TaskCategory.REFACTORING

    @pytest.mark.asyncio
    async def test_refactoring_stays_on_primary_when_it_qualifies(self):
        large_cheap = _make_model(
            name="large_cheap/model",
            cost_per_1k_input=0.001,
            max_context_window=64000,
        )
        large_expensive = _make_model(
            name="large_expensive/model",
            cost_per_1k_input=0.03,
            max_context_window=128000,
        )
        engine = _make_engine([large_cheap, large_expensive])

        messages = [
            {"role": "user", "content": "Refactor this module"},
        ]
        decision = await engine.select_model(messages)
        assert decision.model.name == "large_cheap/model"

    @pytest.mark.asyncio
    async def test_code_review_upgrades_when_primary_too_small(self):
        small = _make_model(
            name="small/model",
            cost_per_1k_input=0.001,
            max_context_window=8000,
            supports_reasoning=True,
        )
        large = _make_model(
            name="large/model",
            cost_per_1k_input=0.01,
            max_context_window=128000,
            supports_reasoning=True,
        )
        engine = _make_engine([small, large])

        messages = [
            {"role": "user", "content": "Review this code for issues"},
        ]
        decision = await engine.select_model(messages)
        assert decision.model.name == "large/model"

    @pytest.mark.asyncio
    async def test_migration_upgrades_when_primary_lacks_reasoning_or_context(self):
        small = _make_model(
            name="small/model",
            cost_per_1k_input=0.001,
            max_context_window=8000,
            supports_reasoning=False,
        )
        capable = _make_model(
            name="capable/model",
            cost_per_1k_input=0.01,
            max_context_window=128000,
            supports_reasoning=True,
        )
        engine = _make_engine([small, capable])

        messages = [
            {"role": "user", "content": "Migrate this from Python 2 to Python 3"},
        ]
        decision = await engine.select_model(messages)
        assert decision.model.name == "capable/model"

    @pytest.mark.asyncio
    async def test_migration_stays_on_primary_when_it_qualifies(self):
        capable_cheap = _make_model(
            name="capable_cheap/model",
            cost_per_1k_input=0.001,
            max_context_window=64000,
            supports_reasoning=True,
            is_local=True,
        )
        capable_expensive = _make_model(
            name="capable_expensive/model",
            cost_per_1k_input=0.03,
            max_context_window=128000,
            supports_reasoning=True,
        )
        engine = _make_engine([capable_cheap, capable_expensive])

        messages = [
            {"role": "user", "content": "Migrate this from Python 2 to Python 3"},
        ]
        decision = await engine.select_model(messages)
        assert decision.model.name == "capable_cheap/model"

    @pytest.mark.asyncio
    async def test_debugging_stays_on_primary_when_it_supports_reasoning(self):
        cheap = _make_model(
            name="cheap/model",
            cost_per_1k_input=0.001,
            supports_reasoning=True,
        )
        expensive = _make_model(
            name="expensive/model",
            cost_per_1k_input=0.03,
            supports_reasoning=True,
        )
        engine = _make_engine([cheap, expensive])

        messages = [
            {"role": "user", "content": "Fix this error in my code"},
        ]
        decision = await engine.select_model(messages)
        assert decision.model.name == "cheap/model"

    @pytest.mark.asyncio
    async def test_debugging_upgrades_when_primary_lacks_reasoning(self):
        cheap = _make_model(
            name="cheap/model",
            cost_per_1k_input=0.001,
            supports_reasoning=False,
        )
        reasoning = _make_model(
            name="reasoning/model",
            cost_per_1k_input=0.01,
            supports_reasoning=True,
        )
        engine = _make_engine([cheap, reasoning])

        messages = [
            {"role": "user", "content": "Fix this error in my code"},
        ]
        decision = await engine.select_model(messages)
        assert decision.model.name == "reasoning/model"

    @pytest.mark.asyncio
    async def test_optimization_upgrades_when_primary_lacks_reasoning(self):
        cheap = _make_model(
            name="cheap/model",
            cost_per_1k_input=0.001,
            supports_reasoning=False,
        )
        reasoning = _make_model(
            name="reasoning/model",
            cost_per_1k_input=0.01,
            supports_reasoning=True,
        )
        engine = _make_engine([cheap, reasoning])

        messages = [
            {"role": "user", "content": "Optimize this function for performance"},
        ]
        decision = await engine.select_model(messages)
        assert decision.model.name == "reasoning/model"

    @pytest.mark.asyncio
    async def test_code_review_upgrades_for_reasoning_and_context(self):
        small_no_reason = _make_model(
            name="small/model",
            cost_per_1k_input=0.001,
            max_context_window=8000,
            supports_reasoning=False,
        )
        large_reasoning = _make_model(
            name="large_reasoning/model",
            cost_per_1k_input=0.01,
            max_context_window=128000,
            supports_reasoning=True,
        )
        engine = _make_engine([small_no_reason, large_reasoning])

        messages = [
            {"role": "user", "content": "Review this code for issues"},
        ]
        decision = await engine.select_model(messages)
        assert decision.model.name == "large_reasoning/model"

    @pytest.mark.asyncio
    async def test_falls_back_to_primary_when_no_model_meets_requirements(self):
        small = _make_model(
            name="small/model",
            cost_per_1k_input=0.001,
            max_context_window=4000,
        )
        engine = _make_engine([small])

        messages = [
            {"role": "user", "content": "Refactor this entire codebase"},
        ]
        decision = await engine.select_model(messages)
        assert decision.model.name == "small/model"

    @pytest.mark.asyncio
    async def test_picks_cheapest_among_qualifying_when_primary_unfit(self):
        small = _make_model(
            name="small/model",
            cost_per_1k_input=0.001,
            max_context_window=8000,
        )
        large_cheap = _make_model(
            name="large_cheap/model",
            cost_per_1k_input=0.005,
            max_context_window=64000,
        )
        large_expensive = _make_model(
            name="large_expensive/model",
            cost_per_1k_input=0.03,
            max_context_window=128000,
        )
        engine = _make_engine([small, large_cheap, large_expensive])

        messages = [
            {"role": "user", "content": "Write tests for this service"},
        ]
        decision = await engine.select_model(messages)
        assert decision.model.name == "large_cheap/model"

    @pytest.mark.asyncio
    async def test_general_task_stays_on_primary_when_confident(self):
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001)
        expensive = _make_model(name="expensive/model", cost_per_1k_input=0.03)
        engine = _make_engine([cheap, expensive], confidence_threshold=0.0)

        messages = [
            {"role": "user", "content": "Tell me a joke"},
        ]
        decision = await engine.select_model(messages)
        assert decision.model.name == "cheap/model"

    @pytest.mark.asyncio
    async def test_explicit_primary_respected_when_it_meets_requirements(self):
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001)
        expensive = _make_model(
            name="expensive/model",
            cost_per_1k_input=0.03,
            max_context_window=128000,
        )
        engine = _make_engine([cheap, expensive], primary_model="expensive/model")

        messages = [
            {"role": "user", "content": "Refactor this module"},
        ]
        decision = await engine.select_model(messages)
        assert decision.model.name == "expensive/model"

    @pytest.mark.asyncio
    async def test_explicit_primary_respected_for_reasoning_task(self):
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001)
        expensive = _make_model(
            name="expensive/model",
            cost_per_1k_input=0.03,
            supports_reasoning=True,
        )
        engine = _make_engine([cheap, expensive], primary_model="expensive/model")

        messages = [
            {"role": "user", "content": "Debug this crash"},
        ]
        decision = await engine.select_model(messages)
        assert decision.model.name == "expensive/model"

    @pytest.mark.asyncio
    async def test_decision_includes_category_and_confidence(self):
        cheap = _make_model(
            name="cheap/model",
            cost_per_1k_input=0.001,
            supports_reasoning=True,
        )
        engine = _make_engine([cheap])

        messages = [
            {"role": "user", "content": "Fix this error in my code"},
        ]
        decision = await engine.select_model(messages)
        assert decision.category == TaskCategory.DEBUGGING
        assert decision.confidence > 0.0
        assert decision.feature_type == FeatureType.CHAT


class TestConfidenceBasedEscalation:
    @pytest.mark.asyncio
    async def test_low_confidence_escalates_to_next_model(self):
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001)
        mid = _make_model(name="mid/model", cost_per_1k_input=0.01)
        expensive = _make_model(name="expensive/model", cost_per_1k_input=0.03)
        engine = _make_engine([cheap, mid, expensive], confidence_threshold=0.9)

        messages = [{"role": "user", "content": "hello world"}]
        decision = await engine.select_model(messages)
        assert decision.model.name == "mid/model"
        assert decision.escalated is True

    @pytest.mark.asyncio
    async def test_high_confidence_does_not_escalate(self):
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001)
        expensive = _make_model(name="expensive/model", cost_per_1k_input=0.03)
        engine = _make_engine([cheap, expensive], confidence_threshold=0.0)

        messages = [{"role": "user", "content": "hello world"}]
        decision = await engine.select_model(messages)
        assert decision.model.name == "cheap/model"
        assert decision.escalated is False

    @pytest.mark.asyncio
    async def test_escalation_with_single_model_uses_it(self):
        only = _make_model(name="only/model", cost_per_1k_input=0.001)
        engine = _make_engine([only], confidence_threshold=0.9)

        messages = [{"role": "user", "content": "hello world"}]
        decision = await engine.select_model(messages)
        assert decision.model.name == "only/model"
        assert decision.escalated is False

    @pytest.mark.asyncio
    async def test_escalation_respects_requirements(self):
        small = _make_model(
            name="small/model",
            cost_per_1k_input=0.001,
            max_context_window=8000,
            supports_reasoning=True,
        )
        mid = _make_model(
            name="mid/model",
            cost_per_1k_input=0.01,
            max_context_window=64000,
            supports_reasoning=True,
        )
        large = _make_model(
            name="large/model",
            cost_per_1k_input=0.03,
            max_context_window=128000,
            supports_reasoning=True,
        )
        engine = _make_engine([small, mid, large], confidence_threshold=0.9)

        messages = [
            {"role": "user", "content": "Refactor this entire codebase"},
        ]
        decision = await engine.select_model(messages)
        assert decision.model.name == "large/model"
        assert decision.escalated is True

    @pytest.mark.asyncio
    async def test_completion_never_escalates(self):
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001)
        expensive = _make_model(name="expensive/model", cost_per_1k_input=0.03)
        engine = _make_engine([cheap, expensive], confidence_threshold=0.9)

        messages = [{"role": "user", "content": "x"}]
        decision = await engine.select_model(messages, max_tokens=50, temperature=0.0)
        assert decision.model.name == "cheap/model"
        assert decision.escalated is False
        assert decision.category == TaskCategory.COMPLETION

    @pytest.mark.asyncio
    async def test_escalation_flag_false_when_confident(self):
        cheap = _make_model(
            name="cheap/model",
            cost_per_1k_input=0.001,
            supports_reasoning=True,
        )
        expensive = _make_model(
            name="expensive/model",
            cost_per_1k_input=0.03,
            supports_reasoning=True,
        )
        engine = _make_engine([cheap, expensive], confidence_threshold=0.5)

        messages = [
            {"role": "user", "content": "Fix this error in my code"},
        ]
        decision = await engine.select_model(messages)
        assert decision.model.name == "cheap/model"
        assert decision.escalated is False

    @pytest.mark.asyncio
    async def test_escalation_skips_cheapest_picks_second(self):
        a = _make_model(name="a/model", cost_per_1k_input=0.001)
        b = _make_model(name="b/model", cost_per_1k_input=0.005)
        c = _make_model(name="c/model", cost_per_1k_input=0.01)
        engine = _make_engine([a, b, c], confidence_threshold=0.9)

        messages = [{"role": "user", "content": "Tell me about distributed systems"}]
        decision = await engine.select_model(messages)
        assert decision.model.name == "b/model"
        assert decision.escalated is True


class TestPrimary:
    def test_primary_property(self):
        model = _make_model(name="primary/model")
        engine = _make_engine([model])
        assert engine.primary.name == "primary/model"

    def test_invalid_primary_raises(self):
        model = _make_model(name="real/model")
        with pytest.raises(ValueError, match="not found"):
            _make_engine([model], primary_model="nonexistent/model")

    def test_empty_registry_raises(self):
        with pytest.raises(ValueError, match="No models available"):
            _make_engine([])


class TestFallbackOrder:
    def test_fallback_orders_by_cost(self):
        cheap = _make_model(name="cheap", cost_per_1k_input=0.001)
        mid = _make_model(name="mid", cost_per_1k_input=0.01)
        expensive = _make_model(name="expensive", cost_per_1k_input=0.03)
        engine = _make_engine([expensive, cheap, mid])

        fallbacks = engine.fallback_order(cheap)
        names = [f.name for f in fallbacks]
        assert names == ["mid", "expensive"]

    def test_fallback_excludes_primary(self):
        a = _make_model(name="a", cost_per_1k_input=0.001)
        b = _make_model(name="b", cost_per_1k_input=0.01)
        engine = _make_engine([a, b])

        fallbacks = engine.fallback_order(a)
        assert all(f.name != "a" for f in fallbacks)

    def test_fallback_single_model_returns_empty(self):
        model = _make_model(name="only/model")
        engine = _make_engine([model])

        fallbacks = engine.fallback_order(model)
        assert fallbacks == []

    def test_fallback_local_before_cloud(self):
        cloud_cheap = _make_model(name="cloud", cost_per_1k_input=0.001, is_local=False)
        local = _make_model(name="local", cost_per_1k_input=0.0, is_local=True)
        cloud_expensive = _make_model(name="cloud2", cost_per_1k_input=0.03, is_local=False)
        engine = _make_engine([cloud_expensive, cloud_cheap, local])

        fallbacks = engine.fallback_order(local)
        names = [f.name for f in fallbacks]
        assert names == ["cloud", "cloud2"]


class TestLLMJudgeIntegration:
    @pytest.mark.asyncio
    @patch("app.router.llm_judge.litellm")
    async def test_judge_triggers_on_low_confidence(self, mock_litellm):
        mock_response = MagicMock()
        mock_response.choices = [
            MagicMock(message=MagicMock(content=json.dumps({"category": "debugging"})))
        ]
        mock_litellm.acompletion = AsyncMock(return_value=mock_response)

        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001, supports_reasoning=True)
        judge = LLMJudge(model="ollama/llama3")
        engine = _make_engine([cheap], judge=judge, confidence_threshold=0.9)

        messages = [{"role": "user", "content": "Can you help me with this code?"}]
        decision = await engine.select_model(messages)

        mock_litellm.acompletion.assert_called_once()
        assert decision.category == TaskCategory.DEBUGGING

    @pytest.mark.asyncio
    async def test_judge_skips_when_confidence_above_threshold(self):
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001, supports_reasoning=True)
        judge = LLMJudge(model="ollama/llama3")
        engine = _make_engine([cheap], judge=judge, confidence_threshold=0.1)

        messages = [{"role": "user", "content": "Fix this error in my code"}]
        decision = await engine.select_model(messages)
        assert decision.category == TaskCategory.DEBUGGING

    @pytest.mark.asyncio
    async def test_judge_skips_for_completions(self):
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001)
        judge = LLMJudge(model="ollama/llama3")
        engine = _make_engine([cheap], judge=judge, confidence_threshold=0.9)

        messages = [{"role": "user", "content": "x"}]
        decision = await engine.select_model(messages, max_tokens=50, temperature=0.0)
        assert decision.category == TaskCategory.COMPLETION

    @pytest.mark.asyncio
    @patch("app.router.llm_judge.litellm")
    async def test_judge_failure_falls_back_to_heuristics(self, mock_litellm):
        mock_litellm.acompletion = AsyncMock(side_effect=Exception("connection failed"))

        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001)
        judge = LLMJudge(model="ollama/llama3")
        engine = _make_engine([cheap], judge=judge, confidence_threshold=0.9)

        messages = [{"role": "user", "content": "Can you help me with this code?"}]
        decision = await engine.select_model(messages)
        assert decision.model.name == "cheap/model"

    @pytest.mark.asyncio
    async def test_no_judge_uses_heuristics_only(self):
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001, supports_reasoning=True)
        engine = _make_engine([cheap])

        messages = [{"role": "user", "content": "Fix this error in my code"}]
        decision = await engine.select_model(messages)
        assert decision.category == TaskCategory.DEBUGGING

    @pytest.mark.asyncio
    @patch("app.router.llm_judge.litellm")
    async def test_judge_result_routes_to_correct_model(self, mock_litellm):
        mock_response = MagicMock()
        mock_response.choices = [
            MagicMock(message=MagicMock(content=json.dumps({"category": "refactoring"})))
        ]
        mock_litellm.acompletion = AsyncMock(return_value=mock_response)

        small = _make_model(name="small/model", cost_per_1k_input=0.001, max_context_window=8000)
        large = _make_model(name="large/model", cost_per_1k_input=0.01, max_context_window=128000)
        judge = LLMJudge(model="ollama/llama3")
        engine = _make_engine([small, large], judge=judge, confidence_threshold=0.9)

        messages = [{"role": "user", "content": "Can you help me with this code?"}]
        decision = await engine.select_model(messages)
        assert decision.category == TaskCategory.REFACTORING
        assert decision.model.name == "large/model"


class TestCentroidClassifierIntegration:
    def _make_centroid_classifier(self, category, confidence):
        mock = MagicMock(spec=CentroidClassifier)
        mock.classify.return_value = ClassificationResult(category=category, confidence=confidence)
        return mock

    @pytest.mark.asyncio
    async def test_centroid_triggers_on_low_heuristic_confidence(self):
        centroid = self._make_centroid_classifier(TaskCategory.DEBUGGING, 0.85)
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001, supports_reasoning=True)
        engine = _make_engine([cheap], confidence_threshold=0.5, centroid_classifier=centroid)

        embedding = np.ones(384, dtype=np.float32)
        messages = [{"role": "user", "content": "hello world"}]
        decision = await engine.select_model(messages, embedding=embedding)

        centroid.classify.assert_called_once()
        assert decision.category == TaskCategory.DEBUGGING

    @pytest.mark.asyncio
    async def test_centroid_skips_when_heuristic_confident(self):
        centroid = self._make_centroid_classifier(TaskCategory.MIGRATION, 0.9)
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001, supports_reasoning=True)
        engine = _make_engine([cheap], confidence_threshold=0.5, centroid_classifier=centroid)

        embedding = np.ones(384, dtype=np.float32)
        messages = [{"role": "user", "content": "Fix this error in my code"}]
        decision = await engine.select_model(messages, embedding=embedding)

        centroid.classify.assert_not_called()
        assert decision.category == TaskCategory.DEBUGGING

    @pytest.mark.asyncio
    async def test_centroid_skips_when_no_embedding(self):
        centroid = self._make_centroid_classifier(TaskCategory.DEBUGGING, 0.9)
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001)
        engine = _make_engine([cheap], confidence_threshold=0.5, centroid_classifier=centroid)

        messages = [{"role": "user", "content": "hello world"}]
        await engine.select_model(messages)

        centroid.classify.assert_not_called()

    @pytest.mark.asyncio
    async def test_centroid_falls_through_to_judge_when_low_confidence(self):
        centroid = self._make_centroid_classifier(TaskCategory.DEBUGGING, 0.3)
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001, supports_reasoning=True)
        judge = MagicMock(spec=LLMJudge)
        judge.classify = AsyncMock(return_value=MagicMock(category=TaskCategory.OPTIMIZATION))

        engine = _make_engine(
            [cheap],
            judge=judge,
            confidence_threshold=0.5,
            centroid_classifier=centroid,
        )

        embedding = np.ones(384, dtype=np.float32)
        messages = [{"role": "user", "content": "hello world"}]
        decision = await engine.select_model(messages, embedding=embedding)

        centroid.classify.assert_called_once()
        judge.classify.assert_called_once()
        assert decision.category == TaskCategory.OPTIMIZATION

    @pytest.mark.asyncio
    async def test_decision_includes_scores(self):
        cheap = _make_model(name="cheap/model", cost_per_1k_input=0.001, supports_reasoning=True)
        engine = _make_engine([cheap])

        messages = [{"role": "user", "content": "Fix this error in my code"}]
        decision = await engine.select_model(messages)

        assert decision.scores is not None
        assert TaskCategory.DEBUGGING in decision.scores

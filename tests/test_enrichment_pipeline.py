from __future__ import annotations

from app.enrichment.context import EnrichmentContext
from app.enrichment.pipeline import EnrichmentPipeline
from app.router.categories import TaskCategory
from app.router.detector import FeatureType


class _AppendEnricher:
    def __init__(self, text: str) -> None:
        self._text = text

    def enrich(self, context: EnrichmentContext) -> EnrichmentContext:
        context.messages.append({"role": "system", "content": self._text})
        return context


class _FailingEnricher:
    def enrich(self, context: EnrichmentContext) -> EnrichmentContext:
        raise RuntimeError("enricher failed")


class TestEnrichmentContext:
    def test_defaults(self):
        ctx = EnrichmentContext()
        assert ctx.messages == []
        assert ctx.category == TaskCategory.GENERAL
        assert ctx.confidence == 0.0
        assert ctx.feature_type == FeatureType.CHAT

    def test_custom_values(self):
        messages = [{"role": "user", "content": "hello"}]
        ctx = EnrichmentContext(
            messages=messages,
            category=TaskCategory.DEBUGGING,
            confidence=0.9,
            feature_type=FeatureType.COMPLETION,
        )
        assert ctx.messages == messages
        assert ctx.category == TaskCategory.DEBUGGING
        assert ctx.confidence == 0.9
        assert ctx.feature_type == FeatureType.COMPLETION

    def test_messages_are_mutable(self):
        ctx = EnrichmentContext(messages=[{"role": "user", "content": "hi"}])
        ctx.messages.append({"role": "system", "content": "added"})
        assert len(ctx.messages) == 2


class TestEnrichmentPipeline:
    def test_empty_pipeline_returns_context_unchanged(self):
        pipeline = EnrichmentPipeline([])
        ctx = EnrichmentContext(
            messages=[{"role": "user", "content": "hello"}],
            category=TaskCategory.GENERATION,
            confidence=0.8,
        )
        result = pipeline.run(ctx)
        assert result.messages == [{"role": "user", "content": "hello"}]
        assert result.category == TaskCategory.GENERATION

    def test_single_enricher_modifies_context(self):
        pipeline = EnrichmentPipeline([_AppendEnricher("step 1")])
        ctx = EnrichmentContext(messages=[{"role": "user", "content": "hello"}])
        result = pipeline.run(ctx)
        assert len(result.messages) == 2
        assert result.messages[1]["content"] == "step 1"

    def test_enrichers_run_in_sequence(self):
        pipeline = EnrichmentPipeline(
            [
                _AppendEnricher("first"),
                _AppendEnricher("second"),
            ]
        )
        ctx = EnrichmentContext(messages=[{"role": "user", "content": "hello"}])
        result = pipeline.run(ctx)
        assert len(result.messages) == 3
        assert result.messages[1]["content"] == "first"
        assert result.messages[2]["content"] == "second"

    def test_failing_enricher_passes_context_through(self):
        pipeline = EnrichmentPipeline(
            [
                _FailingEnricher(),
                _AppendEnricher("after failure"),
            ]
        )
        ctx = EnrichmentContext(messages=[{"role": "user", "content": "hello"}])
        result = pipeline.run(ctx)
        assert len(result.messages) == 2
        assert result.messages[1]["content"] == "after failure"

    def test_all_enrichers_fail_returns_original_context(self):
        pipeline = EnrichmentPipeline([_FailingEnricher(), _FailingEnricher()])
        ctx = EnrichmentContext(messages=[{"role": "user", "content": "hello"}])
        result = pipeline.run(ctx)
        assert result.messages == [{"role": "user", "content": "hello"}]

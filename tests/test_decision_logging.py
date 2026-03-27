from __future__ import annotations

import os
import tempfile
from datetime import datetime

import pytest

from app.logging.models import DecisionRecord
from app.logging.sqlite import SQLiteDecisionRepository


def _make_record(**overrides) -> DecisionRecord:
    defaults = dict(
        timestamp=datetime(2026, 3, 27, 12, 0, 0),
        prompt_hash="abc123",
        category="debugging",
        confidence=0.8,
        feature_type="chat",
        selected_model="openai/gpt-4o-mini",
        used_model="openai/gpt-4o-mini",
        response_time_ms=500,
    )
    defaults.update(overrides)
    return DecisionRecord(**defaults)


class TestDecisionRecord:
    def test_required_fields(self):
        record = _make_record()
        assert record.category == "debugging"
        assert record.confidence == 0.8
        assert record.response_time_ms == 500

    def test_optional_fields_default_to_none(self):
        record = _make_record()
        assert record.input_tokens is None
        assert record.output_tokens is None
        assert record.cost is None
        assert record.rule_votes is None
        assert record.embedding is None

    def test_fallback_triggered_defaults_to_false(self):
        record = _make_record()
        assert record.fallback_triggered is False

    def test_all_fields_set(self):
        record = _make_record(
            input_tokens=100,
            output_tokens=200,
            cost=0.005,
            fallback_triggered=True,
            rule_votes={"debugging": 0.8, "refactoring": 0.2},
            embedding=b"\x00\x01\x02",
        )
        assert record.input_tokens == 100
        assert record.output_tokens == 200
        assert record.cost == 0.005
        assert record.fallback_triggered is True
        assert record.rule_votes == {"debugging": 0.8, "refactoring": 0.2}
        assert record.embedding == b"\x00\x01\x02"


class TestSQLiteDecisionRepository:
    @pytest.fixture
    def db_path(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            yield os.path.join(tmpdir, "test_decisions.db")

    @pytest.fixture
    def repo(self, db_path):
        return SQLiteDecisionRepository(db_path=db_path)

    async def test_save_and_count(self, repo):
        assert await repo.count() == 0
        await repo.save(_make_record())
        assert await repo.count() == 1

    async def test_save_and_get_recent(self, repo):
        record = _make_record(
            input_tokens=50,
            output_tokens=100,
            cost=0.002,
            fallback_triggered=True,
            rule_votes={"debugging": 0.8, "general": 0.2},
        )
        await repo.save(record)

        results = await repo.get_recent(limit=10)
        assert len(results) == 1
        result = results[0]
        assert result.prompt_hash == "abc123"
        assert result.category == "debugging"
        assert result.confidence == 0.8
        assert result.feature_type == "chat"
        assert result.selected_model == "openai/gpt-4o-mini"
        assert result.used_model == "openai/gpt-4o-mini"
        assert result.response_time_ms == 500
        assert result.input_tokens == 50
        assert result.output_tokens == 100
        assert result.cost == 0.002
        assert result.fallback_triggered is True
        assert result.rule_votes == {"debugging": 0.8, "general": 0.2}

    async def test_get_recent_ordering(self, repo):
        await repo.save(_make_record(timestamp=datetime(2026, 1, 1), prompt_hash="first"))
        await repo.save(_make_record(timestamp=datetime(2026, 3, 1), prompt_hash="third"))
        await repo.save(_make_record(timestamp=datetime(2026, 2, 1), prompt_hash="second"))

        results = await repo.get_recent(limit=10)
        assert [r.prompt_hash for r in results] == ["third", "second", "first"]

    async def test_get_recent_respects_limit(self, repo):
        for i in range(5):
            await repo.save(_make_record(prompt_hash=f"hash_{i}"))
        results = await repo.get_recent(limit=2)
        assert len(results) == 2

    async def test_save_with_none_optional_fields(self, repo):
        await repo.save(_make_record())
        results = await repo.get_recent(limit=1)
        result = results[0]
        assert result.input_tokens is None
        assert result.output_tokens is None
        assert result.cost is None
        assert result.rule_votes is None
        assert result.embedding is None

    async def test_save_with_embedding(self, repo):
        embedding_bytes = b"\x00" * 384 * 4
        await repo.save(_make_record(embedding=embedding_bytes))
        results = await repo.get_recent(limit=1)
        assert results[0].embedding == embedding_bytes

    async def test_get_embeddings(self, repo):
        await repo.save(_make_record(prompt_hash="with_emb", embedding=b"\x01\x02"))
        await repo.save(_make_record(prompt_hash="no_emb", embedding=None))

        embeddings = await repo.get_embeddings()
        assert len(embeddings) == 1
        assert embeddings[0] == ("with_emb", b"\x01\x02")

    async def test_get_embeddings_empty(self, repo):
        embeddings = await repo.get_embeddings()
        assert embeddings == []

    async def test_creates_directory_on_first_write(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            db_path = os.path.join(tmpdir, "nested", "dir", "decisions.db")
            repo = SQLiteDecisionRepository(db_path=db_path)
            await repo.save(_make_record())
            assert await repo.count() == 1

    async def test_timestamp_roundtrip(self, repo):
        ts = datetime(2026, 6, 15, 14, 30, 45)
        await repo.save(_make_record(timestamp=ts))
        results = await repo.get_recent(limit=1)
        assert results[0].timestamp == ts

    async def test_fallback_triggered_roundtrip(self, repo):
        await repo.save(_make_record(fallback_triggered=False))
        await repo.save(_make_record(fallback_triggered=True, prompt_hash="fb"))

        results = await repo.get_recent(limit=10)
        by_hash = {r.prompt_hash: r for r in results}
        assert by_hash["abc123"].fallback_triggered is False
        assert by_hash["fb"].fallback_triggered is True

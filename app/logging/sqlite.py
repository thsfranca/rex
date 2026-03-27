from __future__ import annotations

import asyncio
import json
import sqlite3
from datetime import datetime
from pathlib import Path

from app.logging.models import DecisionRecord

_CREATE_TABLE = """
CREATE TABLE IF NOT EXISTS decisions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    prompt_hash TEXT NOT NULL,
    category TEXT NOT NULL,
    confidence REAL NOT NULL,
    feature_type TEXT NOT NULL,
    selected_model TEXT NOT NULL,
    used_model TEXT NOT NULL,
    response_time_ms INTEGER NOT NULL,
    input_tokens INTEGER,
    output_tokens INTEGER,
    cost REAL,
    fallback_triggered INTEGER NOT NULL DEFAULT 0,
    rule_votes TEXT,
    embedding BLOB
);
"""

_CREATE_INDEXES = [
    "CREATE INDEX IF NOT EXISTS idx_decisions_timestamp ON decisions(timestamp);",
    "CREATE INDEX IF NOT EXISTS idx_decisions_category ON decisions(category);",
]


class SQLiteDecisionRepository:
    def __init__(self, db_path: str = "~/.rex/decisions.db") -> None:
        self._db_path = str(Path(db_path).expanduser())
        self._initialized = False

    def _get_connection(self) -> sqlite3.Connection:
        Path(self._db_path).parent.mkdir(parents=True, exist_ok=True)
        conn = sqlite3.connect(self._db_path)
        conn.row_factory = sqlite3.Row
        if not self._initialized:
            conn.execute(_CREATE_TABLE)
            for idx_sql in _CREATE_INDEXES:
                conn.execute(idx_sql)
            conn.commit()
            self._initialized = True
        return conn

    def _save_sync(self, record: DecisionRecord) -> None:
        conn = self._get_connection()
        try:
            conn.execute(
                """
                INSERT INTO decisions (
                    timestamp, prompt_hash, category, confidence, feature_type,
                    selected_model, used_model, response_time_ms,
                    input_tokens, output_tokens, cost,
                    fallback_triggered, rule_votes, embedding
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    record.timestamp.isoformat(),
                    record.prompt_hash,
                    record.category,
                    record.confidence,
                    record.feature_type,
                    record.selected_model,
                    record.used_model,
                    record.response_time_ms,
                    record.input_tokens,
                    record.output_tokens,
                    record.cost,
                    int(record.fallback_triggered),
                    json.dumps(record.rule_votes) if record.rule_votes is not None else None,
                    record.embedding,
                ),
            )
            conn.commit()
        finally:
            conn.close()

    def _get_recent_sync(self, limit: int) -> list[DecisionRecord]:
        conn = self._get_connection()
        try:
            rows = conn.execute(
                "SELECT * FROM decisions ORDER BY timestamp DESC LIMIT ?",
                (limit,),
            ).fetchall()
            return [self._row_to_record(row) for row in rows]
        finally:
            conn.close()

    def _count_sync(self) -> int:
        conn = self._get_connection()
        try:
            row = conn.execute("SELECT COUNT(*) FROM decisions").fetchone()
            return row[0]
        finally:
            conn.close()

    def _get_embeddings_sync(self) -> list[tuple[str, bytes]]:
        conn = self._get_connection()
        try:
            rows = conn.execute(
                "SELECT prompt_hash, embedding FROM decisions WHERE embedding IS NOT NULL"
            ).fetchall()
            return [(row["prompt_hash"], row["embedding"]) for row in rows]
        finally:
            conn.close()

    @staticmethod
    def _row_to_record(row: sqlite3.Row) -> DecisionRecord:
        rule_votes_raw = row["rule_votes"]
        rule_votes = json.loads(rule_votes_raw) if rule_votes_raw is not None else None

        return DecisionRecord(
            timestamp=datetime.fromisoformat(row["timestamp"]),
            prompt_hash=row["prompt_hash"],
            category=row["category"],
            confidence=row["confidence"],
            feature_type=row["feature_type"],
            selected_model=row["selected_model"],
            used_model=row["used_model"],
            response_time_ms=row["response_time_ms"],
            input_tokens=row["input_tokens"],
            output_tokens=row["output_tokens"],
            cost=row["cost"],
            fallback_triggered=bool(row["fallback_triggered"]),
            rule_votes=rule_votes,
            embedding=row["embedding"],
        )

    async def save(self, record: DecisionRecord) -> None:
        await asyncio.to_thread(self._save_sync, record)

    async def get_recent(self, limit: int) -> list[DecisionRecord]:
        return await asyncio.to_thread(self._get_recent_sync, limit)

    async def count(self) -> int:
        return await asyncio.to_thread(self._count_sync)

    def _get_rule_votes_sync(self) -> dict[str, dict[str, float]]:
        conn = self._get_connection()
        try:
            rows = conn.execute(
                "SELECT prompt_hash, rule_votes FROM decisions WHERE rule_votes IS NOT NULL"
            ).fetchall()
            result = {}
            for row in rows:
                votes = json.loads(row["rule_votes"])
                if votes:
                    result[row["prompt_hash"]] = votes
            return result
        finally:
            conn.close()

    async def get_embeddings(self) -> list[tuple[str, bytes]]:
        return await asyncio.to_thread(self._get_embeddings_sync)

    async def get_rule_votes(self) -> dict[str, dict[str, float]]:
        return await asyncio.to_thread(self._get_rule_votes_sync)

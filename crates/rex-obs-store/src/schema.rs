pub const SCHEMA_VERSION: u32 = 1;

pub const CREATE_TABLES_V1: &str = r#"
CREATE TABLE IF NOT EXISTS config_snapshots (
    id TEXT PRIMARY KEY,
    payload_json TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS streams (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id TEXT NOT NULL REFERENCES config_snapshots(id),
    request_id INTEGER NOT NULL,
    trace_id TEXT NOT NULL,
    turn_id TEXT NOT NULL DEFAULT '',
    terminal TEXT NOT NULL,
    route TEXT NOT NULL,
    cache_decision TEXT NOT NULL,
    decision_id TEXT NOT NULL,
    inference_runtime TEXT NOT NULL,
    mode TEXT NOT NULL,
    model TEXT NOT NULL,
    elapsed_ms INTEGER NOT NULL,
    chunks_sent INTEGER NOT NULL,
    prompt_tokens INTEGER NOT NULL,
    context_tokens INTEGER NOT NULL,
    context_candidates INTEGER NOT NULL,
    context_selected INTEGER NOT NULL,
    context_truncated INTEGER NOT NULL,
    retrieval TEXT NOT NULL,
    compression_strategy TEXT NOT NULL,
    cached_tokens INTEGER,
    prefix_hash TEXT,
    parse_retries INTEGER,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS runs (
    run_id TEXT PRIMARY KEY,
    scenario TEXT NOT NULL,
    started_at_ms INTEGER NOT NULL,
    snapshot_id TEXT NOT NULL REFERENCES config_snapshots(id)
);

CREATE TABLE IF NOT EXISTS run_tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL REFERENCES runs(run_id),
    task_id TEXT NOT NULL,
    outcome TEXT NOT NULL
);
"#;

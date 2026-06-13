/// Per-stream economics row emitted at `stream.terminal` (stdout + OTLP).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamEconomicsRecord {
    pub snapshot_id: String,
    pub request_id: u64,
    pub trace_id: String,
    pub turn_id: String,
    pub terminal: String,
    pub route: String,
    pub cache_decision: String,
    pub decision_id: String,
    pub inference_runtime: String,
    pub mode: String,
    pub model: String,
    pub elapsed_ms: u64,
    pub chunks_sent: u64,
    pub prompt_tokens: u64,
    pub context_tokens: u64,
    pub context_candidates: u64,
    pub context_selected: u64,
    pub context_truncated: bool,
    pub retrieval: String,
    pub compression_strategy: String,
    pub cached_tokens: Option<u64>,
    pub prefix_hash: Option<String>,
    pub parse_retries: Option<u64>,
    /// Agent loop terminal class when route uses live sidecar (R064).
    pub agent_loop_terminal: Option<String>,
}

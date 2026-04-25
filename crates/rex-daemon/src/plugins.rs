use std::cmp::Reverse;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

#[derive(Debug, Clone, Copy)]
pub struct TokenBudget {
    pub max_prompt_tokens: usize,
    pub max_context_tokens: usize,
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self {
            max_prompt_tokens: 512,
            max_context_tokens: 192,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BehaviorSnapshot {
    pub typing_cadence_cpm: u16,
    pub pause_events_last_minute: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BehaviorDecision {
    Allow,
    Suppress { reason: &'static str },
}

#[derive(Debug, Clone)]
pub struct ContextRequest {
    pub prompt: String,
    pub diagnostics_hint: Option<String>,
    pub cache_bypass: bool,
    pub behavior_snapshot: BehaviorSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStatus {
    Hit,
    MissStored,
    Bypass,
}

#[derive(Debug, Clone)]
pub struct PipelineMetrics {
    pub prompt_tokens: usize,
    pub selected_context_tokens: usize,
    pub context_candidates: usize,
    pub context_selected: usize,
    pub context_truncated: bool,
    pub cache_status: CacheStatus,
    pub behavior_decision: BehaviorDecision,
}

#[derive(Debug, Clone)]
pub struct PipelineResult {
    pub effective_prompt: String,
    pub metrics: PipelineMetrics,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextChunk {
    pub source: String,
    pub text: String,
}

#[derive(Debug, Clone)]
struct WorkspaceDoc {
    source: String,
    text: String,
}

#[derive(Debug, Clone)]
pub struct LexicalWorkspaceIndexer {
    docs: Vec<WorkspaceDoc>,
}

impl LexicalWorkspaceIndexer {
    pub fn with_seed_docs<I>(docs: I) -> Self
    where
        I: IntoIterator<Item = (String, String)>,
    {
        let docs = docs
            .into_iter()
            .filter(|(source, text)| !source.is_empty() && !text.is_empty())
            .filter(|(source, _)| !is_ignored_path(source))
            .map(|(source, text)| WorkspaceDoc { source, text })
            .collect();
        Self { docs }
    }

    pub fn default_seeded() -> Self {
        Self::with_seed_docs([
            (
                "src/runtime.rs".to_string(),
                "daemon startup shutdown socket lifecycle and error handling".to_string(),
            ),
            (
                "src/transport.rs".to_string(),
                "unix domain socket grpc client connection retry timeout".to_string(),
            ),
            (
                "src/service.rs".to_string(),
                "stream inference chunks done terminal event status endpoint".to_string(),
            ),
            (
                "tests/uds_e2e.rs".to_string(),
                "integration tests startup race interruption stream reliability".to_string(),
            ),
            (
                "docs/CI.md".to_string(),
                "ci checks clippy fmt tests and merge safety".to_string(),
            ),
        ])
    }

    pub fn search(&self, query: &str, top_k: usize) -> Vec<ContextChunk> {
        let query_terms = normalized_terms(query);
        if query_terms.is_empty() || top_k == 0 {
            return Vec::new();
        }
        let mut scored = self
            .docs
            .iter()
            .map(|doc| {
                let score = score_terms(&query_terms, &normalized_terms(&doc.text));
                (score, doc.source.as_str(), doc.text.as_str())
            })
            .filter(|(score, _, _)| *score > 0)
            .collect::<Vec<_>>();
        scored.sort_by_key(|(score, source, _)| (Reverse(*score), *source));
        scored
            .into_iter()
            .take(top_k)
            .map(|(_, source, text)| ContextChunk {
                source: source.to_string(),
                text: text.to_string(),
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct ExtractiveContextCompressor;

impl ExtractiveContextCompressor {
    pub fn compress(&self, chunks: &[ContextChunk], budget_tokens: usize) -> (String, usize, bool) {
        if budget_tokens == 0 || chunks.is_empty() {
            return (String::new(), 0, !chunks.is_empty());
        }
        let mut out = String::new();
        let mut used = 0usize;
        let mut truncated = false;
        for chunk in chunks {
            let line = format!("[{}] {}", chunk.source, chunk.text);
            let line_tokens = estimate_tokens(&line);
            if used + line_tokens > budget_tokens {
                truncated = true;
                break;
            }
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&line);
            used += line_tokens;
        }
        (out, used, truncated)
    }
}

#[derive(Debug, Clone)]
pub struct ExactPrefixCache {
    entries: HashMap<String, CacheEntry>,
    ttl: Duration,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    value: String,
    inserted_at: Instant,
}

impl ExactPrefixCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            ttl,
        }
    }

    pub fn get(&mut self, key: &str) -> Option<String> {
        let now = Instant::now();
        self.entries
            .retain(|_, entry| now.duration_since(entry.inserted_at) <= self.ttl);
        self.entries.get(key).map(|entry| entry.value.clone())
    }

    pub fn put(&mut self, key: String, value: String) {
        self.entries.insert(
            key,
            CacheEntry {
                value,
                inserted_at: Instant::now(),
            },
        );
    }
}

#[derive(Debug, Clone)]
pub struct BehavioralPrefilter;

impl BehavioralPrefilter {
    pub fn evaluate(&self, snapshot: BehaviorSnapshot) -> BehaviorDecision {
        if snapshot.typing_cadence_cpm > 420 && snapshot.pause_events_last_minute == 0 {
            BehaviorDecision::Suppress {
                reason: "focused-typing-window",
            }
        } else {
            BehaviorDecision::Allow
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContextPipeline {
    budget: TokenBudget,
    indexer: LexicalWorkspaceIndexer,
    compressor: ExtractiveContextCompressor,
    cache: ExactPrefixCache,
    behavior: BehavioralPrefilter,
}

impl ContextPipeline {
    pub fn default_sidecar_like() -> Self {
        Self {
            budget: TokenBudget::default(),
            indexer: LexicalWorkspaceIndexer::default_seeded(),
            compressor: ExtractiveContextCompressor,
            cache: ExactPrefixCache::new(Duration::from_secs(600)),
            behavior: BehavioralPrefilter,
        }
    }

    pub fn prepare(&mut self, request: &ContextRequest) -> PipelineResult {
        let bounded_prompt = apply_prompt_budget(&request.prompt, self.budget.max_prompt_tokens);
        let decision = self.behavior.evaluate(request.behavior_snapshot);
        if let BehaviorDecision::Suppress { .. } = decision {
            return PipelineResult {
                effective_prompt: bounded_prompt.clone(),
                metrics: PipelineMetrics {
                    prompt_tokens: estimate_tokens(&bounded_prompt),
                    selected_context_tokens: 0,
                    context_candidates: 0,
                    context_selected: 0,
                    context_truncated: false,
                    cache_status: CacheStatus::Bypass,
                    behavior_decision: decision,
                },
            };
        }

        let cache_key = stable_prefix_key(&bounded_prompt);
        if !request.cache_bypass {
            if let Some(context) = self.cache.get(&cache_key) {
                return PipelineResult {
                    effective_prompt: join_prompt_and_context(&bounded_prompt, &context),
                    metrics: PipelineMetrics {
                        prompt_tokens: estimate_tokens(&bounded_prompt),
                        selected_context_tokens: estimate_tokens(&context),
                        context_candidates: 0,
                        context_selected: 0,
                        context_truncated: false,
                        cache_status: CacheStatus::Hit,
                        behavior_decision: decision,
                    },
                };
            }
        }

        let mut query = bounded_prompt.clone();
        if let Some(diagnostics) = &request.diagnostics_hint {
            query.push(' ');
            query.push_str(diagnostics);
        }
        let candidates = self.indexer.search(&query, 5);
        let candidate_count = candidates.len();
        let (context, selected_tokens, truncated) = self
            .compressor
            .compress(&candidates, self.budget.max_context_tokens);
        if !request.cache_bypass {
            self.cache.put(cache_key, context.clone());
        }
        PipelineResult {
            effective_prompt: join_prompt_and_context(&bounded_prompt, &context),
            metrics: PipelineMetrics {
                prompt_tokens: estimate_tokens(&bounded_prompt),
                selected_context_tokens: selected_tokens,
                context_candidates: candidate_count,
                context_selected: if context.is_empty() {
                    0
                } else {
                    context.lines().count()
                },
                context_truncated: truncated,
                cache_status: if request.cache_bypass {
                    CacheStatus::Bypass
                } else {
                    CacheStatus::MissStored
                },
                behavior_decision: decision,
            },
        }
    }
}

fn is_ignored_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    normalized.contains("/node_modules/")
        || normalized.contains("/target/")
        || normalized.starts_with("target/")
        || normalized.starts_with("node_modules/")
        || normalized.starts_with(".git/")
}

fn stable_prefix_key(prompt: &str) -> String {
    prompt.chars().take(96).collect::<String>()
}

fn apply_prompt_budget(prompt: &str, max_prompt_tokens: usize) -> String {
    prompt
        .chars()
        .take(max_prompt_tokens.saturating_mul(4))
        .collect()
}

fn join_prompt_and_context(prompt: &str, context: &str) -> String {
    if context.is_empty() {
        return prompt.to_string();
    }
    format!("{prompt}\n\n[context]\n{context}")
}

fn normalized_terms(text: &str) -> Vec<String> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .collect()
}

fn score_terms(query_terms: &[String], doc_terms: &[String]) -> usize {
    if query_terms.is_empty() || doc_terms.is_empty() {
        return 0;
    }
    query_terms
        .iter()
        .filter(|query| doc_terms.iter().any(|doc| doc == *query))
        .count()
}

#[cfg(test)]
mod tests {
    use super::{
        stable_prefix_key, BehaviorSnapshot, CacheStatus, ContextPipeline, ContextRequest,
        LexicalWorkspaceIndexer,
    };

    #[test]
    fn lexical_index_returns_deterministic_ranked_results() {
        let index = LexicalWorkspaceIndexer::with_seed_docs([
            ("b.rs".to_string(), "socket retry logic".to_string()),
            ("a.rs".to_string(), "socket retry logic".to_string()),
        ]);
        let hits = index.search("socket retry", 10);
        let ordered_sources = hits
            .into_iter()
            .map(|chunk| chunk.source)
            .collect::<Vec<_>>();
        assert_eq!(
            ordered_sources,
            vec!["a.rs".to_string(), "b.rs".to_string()]
        );
    }

    #[test]
    fn cache_key_uses_stable_prompt_prefix() {
        let a = stable_prefix_key("abcdef");
        let b = stable_prefix_key("abcdef and more");
        assert!(b.starts_with(&a));
    }

    #[test]
    fn pipeline_reports_cache_hit_after_first_request() {
        let mut pipeline = ContextPipeline::default_sidecar_like();
        let request = ContextRequest {
            prompt: "status stream retry".to_string(),
            diagnostics_hint: None,
            cache_bypass: false,
            behavior_snapshot: BehaviorSnapshot::default(),
        };
        let first = pipeline.prepare(&request);
        assert_eq!(first.metrics.cache_status, CacheStatus::MissStored);
        let second = pipeline.prepare(&request);
        assert_eq!(second.metrics.cache_status, CacheStatus::Hit);
    }

    #[test]
    fn pipeline_can_bypass_cache() {
        let mut pipeline = ContextPipeline::default_sidecar_like();
        let request = ContextRequest {
            prompt: "status stream retry".to_string(),
            diagnostics_hint: None,
            cache_bypass: true,
            behavior_snapshot: BehaviorSnapshot::default(),
        };
        let result = pipeline.prepare(&request);
        assert_eq!(result.metrics.cache_status, CacheStatus::Bypass);
    }

    #[test]
    fn behavior_filter_can_suppress_when_user_is_in_focus_flow() {
        let mut pipeline = ContextPipeline::default_sidecar_like();
        let request = ContextRequest {
            prompt: "complete this helper".to_string(),
            diagnostics_hint: None,
            cache_bypass: false,
            behavior_snapshot: BehaviorSnapshot {
                typing_cadence_cpm: 500,
                pause_events_last_minute: 0,
            },
        };
        let result = pipeline.prepare(&request);
        assert!(matches!(
            result.metrics.behavior_decision,
            super::BehaviorDecision::Suppress { .. }
        ));
    }
}

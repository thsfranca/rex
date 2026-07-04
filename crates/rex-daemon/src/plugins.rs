use std::cmp::Reverse;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::adapters::AdapterCapabilities;
use crate::turn_correlation::strip_client_context_blocks;

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

impl TokenBudget {
    pub fn from_config() -> Self {
        let (max_prompt_tokens, max_context_tokens) = crate::settings::get().token_budget();
        Self {
            max_prompt_tokens: max_prompt_tokens.max(1),
            max_context_tokens: max_context_tokens.max(1),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalDecision {
    Ran,
    Skipped,
    AdvisoryBundle,
}

pub struct ContextRequest {
    pub prompt: String,
    pub diagnostics_hint: Option<String>,
    pub cache_bypass: bool,
    pub behavior_snapshot: BehaviorSnapshot,
    /// When true, skip lexical retrieval (directive or gate).
    pub retrieve_off: bool,
    pub active_file_path: Option<String>,
    /// Scopes prefix cache entries per harness terminal (parallel harness).
    pub harness_session_id: String,
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
    pub retrieval: RetrievalDecision,
    pub compression_strategy: &'static str,
    pub knowledge_status: &'static str,
}

#[derive(Debug, Clone)]
pub struct PipelineResult {
    pub effective_prompt: String,
    pub injected_context: String,
    pub injected_paths: Vec<String>,
    pub c1_stripped: bool,
    pub metrics: PipelineMetrics,
}

/// Paths already embedded in daemon context for sidecar skip logic (R065 / R066).
pub fn build_injected_files_manifest(
    active_file_path: Option<&str>,
    injected_context: &str,
    configured_paths: &[String],
) -> Vec<String> {
    let mut paths: Vec<String> = configured_paths.to_vec();
    if let Some(path) = active_file_path
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        paths.push(path.to_string());
    }
    let lower = injected_context.to_ascii_lowercase();
    if lower.contains("readme") || lower.contains("<<tool_result:fs.read>>") {
        paths.push("README.md".to_string());
    }
    paths.sort();
    paths.dedup();
    paths
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

    /// Bounded walk of `REX_WORKSPACE_ROOT` (or cwd) for `.rs`, `.md`, `.toml` files.
    pub fn from_workspace_root(root: &Path) -> Self {
        const MAX_FILES: usize = 32;
        const MAX_BYTES_PER_FILE: usize = 4_096;
        let mut docs = Vec::new();
        collect_workspace_docs(root, root, &mut docs, MAX_FILES, MAX_BYTES_PER_FILE);
        if docs.is_empty() {
            return Self::default_seeded();
        }
        Self::with_seed_docs(docs)
    }

    pub fn from_config() -> Self {
        let loaded = crate::settings::get();
        let mode = loaded.workspace_indexer_mode();
        if mode.trim().eq_ignore_ascii_case("seeded") {
            return Self::default_seeded();
        }
        match loaded.resolve_workspace_root() {
            Ok(root) => Self::from_workspace_root(&root),
            Err(_) => Self { docs: Vec::new() },
        }
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

const RETRIEVAL_SKIP_MAX_PROMPT_CHARS: usize = 48;

const MAX_BYTES_PER_INJECTED_PATH: usize = 4096;
const CONTEXT_PATHS_BUDGET_FRACTION: f32 = 0.30;

pub fn should_skip_retrieval(request: &ContextRequest) -> bool {
    if request.retrieve_off {
        return true;
    }
    if crate::settings::get().advisory_intent_enabled()
        && crate::advisory_intent::matches_advisory_intent(&request.prompt)
    {
        return false;
    }
    if request.prompt.chars().count() <= RETRIEVAL_SKIP_MAX_PROMPT_CHARS {
        return true;
    }
    matches!(
        BehavioralPrefilter::evaluate(request.behavior_snapshot),
        BehaviorDecision::Suppress { .. }
    )
}

fn load_advisory_bundle(budget_tokens: usize) -> (String, usize, bool) {
    let root = match crate::settings::get().resolve_workspace_root() {
        Ok(path) => path,
        Err(_) => return (String::new(), 0, false),
    };
    let rel_paths = crate::doc_manifest::advisory_bundle_paths(&root);
    load_path_bundle(&root, &rel_paths, budget_tokens)
}

fn load_context_paths(budget_tokens: usize) -> (String, usize, Vec<String>) {
    let loaded = crate::settings::get();
    let paths: Vec<String> = loaded.context_paths().to_vec();
    if paths.is_empty() {
        return (String::new(), 0, Vec::new());
    }
    let root = match loaded.resolve_workspace_root() {
        Ok(path) => path,
        Err(_) => return (String::new(), 0, Vec::new()),
    };
    let path_budget = ((budget_tokens as f32) * CONTEXT_PATHS_BUDGET_FRACTION).floor() as usize;
    let (text, tokens, _) = load_path_bundle(&root, &paths, path_budget);
    let injected: Vec<String> = paths
        .iter()
        .filter(|rel| root.join(rel).is_file())
        .cloned()
        .collect();
    (text, tokens, injected)
}

fn load_path_bundle(root: &Path, rel_paths: &[String], budget_tokens: usize) -> (String, usize, bool) {
    let mut lines: Vec<String> = Vec::new();
    for rel in rel_paths {
        let path = root.join(rel);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let snippet: String = text.chars().take(MAX_BYTES_PER_INJECTED_PATH).collect();
        lines.push(format!("[{rel}] {snippet}"));
    }
    if lines.is_empty() {
        return (String::new(), 0, false);
    }
    let mut out = String::new();
    let mut used = 0usize;
    let mut truncated = false;
    for line in lines {
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

impl ExtractiveContextCompressor {
    pub fn compress(
        &self,
        query: &str,
        chunks: &[ContextChunk],
        budget_tokens: usize,
    ) -> (String, usize, bool) {
        if budget_tokens == 0 || chunks.is_empty() {
            return (String::new(), 0, !chunks.is_empty());
        }
        let query_terms = normalized_terms(query);
        let mut ranked = chunks
            .iter()
            .map(|chunk| {
                let line = format!("[{}] {}", chunk.source, chunk.text);
                let score = score_terms(&query_terms, &normalized_terms(&line));
                (score, line)
            })
            .collect::<Vec<_>>();
        ranked.sort_by_key(|(score, line)| (Reverse(*score), line.clone()));
        let mut out = String::new();
        let mut used = 0usize;
        let mut truncated = false;
        for (_, line) in ranked {
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
    pub fn evaluate(snapshot: BehaviorSnapshot) -> BehaviorDecision {
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
}

impl ContextPipeline {
    /// Production pipeline: workspace indexer unless config indexer is `seeded`.
    pub fn production_default() -> Self {
        Self::with_indexer(LexicalWorkspaceIndexer::from_config())
    }

    pub(crate) fn with_indexer(indexer: LexicalWorkspaceIndexer) -> Self {
        Self {
            budget: TokenBudget::from_config(),
            indexer,
            compressor: ExtractiveContextCompressor,
            cache: ExactPrefixCache::new(Duration::from_secs(600)),
        }
    }

    pub fn prepare(
        &mut self,
        request: &ContextRequest,
        capabilities: AdapterCapabilities,
    ) -> PipelineResult {
        let mut bounded_prompt = if capabilities.truncate_prompt {
            apply_prompt_budget(&request.prompt, self.budget.max_prompt_tokens)
        } else {
            request.prompt.clone()
        };
        let mut c1_stripped = false;
        if capabilities.attach_context && !should_skip_retrieval(request) {
            let (stripped, applied) = strip_client_context_blocks(&bounded_prompt);
            if applied {
                bounded_prompt = stripped;
                c1_stripped = true;
            }
        }
        let decision = BehavioralPrefilter::evaluate(request.behavior_snapshot);
        if let BehaviorDecision::Suppress { .. } = decision {
            return PipelineResult {
                effective_prompt: bounded_prompt.clone(),
                injected_context: String::new(),
                injected_paths: Vec::new(),
                c1_stripped,
                metrics: PipelineMetrics {
                    prompt_tokens: estimate_tokens(&bounded_prompt),
                    selected_context_tokens: 0,
                    context_candidates: 0,
                    context_selected: 0,
                    context_truncated: false,
                    cache_status: CacheStatus::Bypass,
                    behavior_decision: decision,
                    retrieval: RetrievalDecision::Skipped,
                    compression_strategy: "none",
                    knowledge_status: "off",
                },
            };
        }

        if !capabilities.attach_context {
            return PipelineResult {
                effective_prompt: bounded_prompt.clone(),
                injected_context: String::new(),
                injected_paths: Vec::new(),
                c1_stripped,
                metrics: PipelineMetrics {
                    prompt_tokens: estimate_tokens(&bounded_prompt),
                    selected_context_tokens: 0,
                    context_candidates: 0,
                    context_selected: 0,
                    context_truncated: false,
                    cache_status: if request.cache_bypass {
                        CacheStatus::Bypass
                    } else {
                        CacheStatus::MissStored
                    },
                    behavior_decision: decision,
                    retrieval: RetrievalDecision::Skipped,
                    compression_strategy: "none",
                    knowledge_status: "off",
                },
            };
        }

        let cache_key = stable_prefix_key(&request.harness_session_id, &bounded_prompt);
        if !request.cache_bypass {
            if let Some(context) = self.cache.get(&cache_key) {
                return PipelineResult {
                    effective_prompt: join_prompt_and_context(&bounded_prompt, &context),
                    injected_context: context.clone(),
                    injected_paths: Vec::new(),
                    c1_stripped,
                    metrics: PipelineMetrics {
                        prompt_tokens: estimate_tokens(&bounded_prompt),
                        selected_context_tokens: estimate_tokens(&context),
                        context_candidates: 0,
                        context_selected: 0,
                        context_truncated: false,
                        cache_status: CacheStatus::Hit,
                        behavior_decision: decision,
                        retrieval: RetrievalDecision::Skipped,
                        compression_strategy: "prefix_hit",
                        knowledge_status: "off",
                    },
                };
            }
        }

        let (paths_context, paths_tokens, injected_paths) =
            load_context_paths(self.budget.max_context_tokens);
        let remaining_budget = self
            .budget
            .max_context_tokens
            .saturating_sub(paths_tokens);

        let retrieval = if should_skip_retrieval(request) {
            RetrievalDecision::Skipped
        } else if crate::settings::get().advisory_intent_enabled()
            && crate::advisory_intent::matches_advisory_intent(&bounded_prompt)
            && bounded_prompt.chars().count() <= RETRIEVAL_SKIP_MAX_PROMPT_CHARS
        {
            RetrievalDecision::AdvisoryBundle
        } else {
            RetrievalDecision::Ran
        };
        let mut query = bounded_prompt.clone();
        if let Some(diagnostics) = &request.diagnostics_hint {
            query.push(' ');
            query.push_str(diagnostics);
        }
        let (mut context, mut selected_tokens, truncated, candidate_count, compression_strategy) =
            if retrieval == RetrievalDecision::AdvisoryBundle {
                let root = crate::settings::get()
                    .resolve_workspace_root()
                    .unwrap_or_else(|_| PathBuf::from("."));
                let bundle_len = crate::doc_manifest::advisory_bundle_paths(&root).len();
                let (bundle, tokens, trunc) = load_advisory_bundle(remaining_budget);
                (
                    bundle,
                    paths_tokens + tokens,
                    trunc,
                    bundle_len,
                    "advisory_bundle",
                )
            } else if retrieval == RetrievalDecision::Ran {
                let mut candidates = self.indexer.search(&query, 5);
                if let Some(active) = &request.active_file_path {
                    boost_active_file_candidate(&mut candidates, active);
                }
                let candidate_count = candidates.len();
                let (ctx, tokens, trunc) = self.compressor.compress(
                    &query,
                    &candidates,
                    remaining_budget,
                );
                let strategy = if candidates.is_empty() {
                    "none"
                } else {
                    "extractive_query"
                };
                (
                    ctx,
                    paths_tokens + tokens,
                    trunc,
                    candidate_count,
                    strategy,
                )
            } else {
                (
                    String::new(),
                    paths_tokens,
                    false,
                    0,
                    if paths_context.is_empty() {
                        "none"
                    } else {
                        "context_paths"
                    },
                )
            };

        if !paths_context.is_empty() {
            if context.is_empty() {
                context = paths_context;
            } else {
                context = format!("{paths_context}\n{context}");
            }
        }

        let knowledge_status = if crate::settings::get().knowledge_enabled() {
            let root = crate::settings::get()
                .resolve_workspace_root()
                .unwrap_or_else(|_| PathBuf::from("."));
            let knowledge = crate::knowledge::load_knowledge_context(
                &root,
                self.budget.max_context_tokens,
            );
            if !knowledge.text.is_empty() {
                if context.is_empty() {
                    context = knowledge.text;
                } else {
                    context = format!("{}\n{}", context, knowledge.text);
                }
                selected_tokens += knowledge.tokens;
            }
            knowledge.status
        } else {
            "off"
        };

        if !request.cache_bypass {
            self.cache.put(cache_key, context.clone());
        }
        PipelineResult {
            effective_prompt: join_prompt_and_context(&bounded_prompt, &context),
            injected_context: context.clone(),
            injected_paths,
            c1_stripped,
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
                retrieval,
                compression_strategy,
                knowledge_status,
            },
        }
    }
}

impl RetrievalDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ran => "ran",
            Self::Skipped => "skipped",
            Self::AdvisoryBundle => "advisory_bundle",
        }
    }
}

fn boost_active_file_candidate(candidates: &mut Vec<ContextChunk>, active_path: &str) {
    let active_norm = active_path.replace('\\', "/");
    let Some(idx) = candidates.iter().position(|c| {
        let source_norm = c.source.replace('\\', "/");
        active_norm.ends_with(&source_norm) || source_norm.ends_with(&active_norm)
    }) else {
        return;
    };
    if idx > 0 {
        let chunk = candidates.remove(idx);
        candidates.insert(0, chunk);
    }
}

fn collect_workspace_docs(
    root: &Path,
    dir: &Path,
    out: &mut Vec<(String, String)>,
    max_files: usize,
    max_bytes: usize,
) {
    if out.len() >= max_files {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        if out.len() >= max_files {
            break;
        }
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if is_ignored_path(&rel) {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.is_dir() {
            collect_workspace_docs(root, &path, out, max_files, max_bytes);
            continue;
        }
        if !is_indexable_extension(&rel) {
            continue;
        }
        if meta.len() as usize > max_bytes {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let trimmed = text.chars().take(max_bytes).collect::<String>();
        if !trimmed.is_empty() {
            out.push((rel, trimmed));
        }
    }
}

fn is_indexable_extension(path: &str) -> bool {
    path.ends_with(".rs")
        || path.ends_with(".md")
        || path.ends_with(".toml")
        || path.ends_with(".proto")
}

fn is_ignored_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    normalized.contains("/node_modules/")
        || normalized.contains("/target/")
        || normalized.starts_with("target/")
        || normalized.starts_with("node_modules/")
        || normalized.starts_with(".git/")
}

fn stable_prefix_key(harness_session_id: &str, prompt: &str) -> String {
    let prefix = prompt.chars().take(96).collect::<String>();
    let session = harness_session_id.trim();
    if session.is_empty() {
        prefix
    } else {
        format!("{session}:{prefix}")
    }
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
    use std::sync::Arc;

    use super::{
        build_injected_files_manifest, load_advisory_bundle, stable_prefix_key, BehaviorSnapshot,
        CacheStatus, ContextChunk, ContextPipeline, ContextRequest, ExtractiveContextCompressor,
        LexicalWorkspaceIndexer, TokenBudget,
    };
    use crate::adapters::{AdapterCapabilities, RuntimeKind};
    use super::should_skip_retrieval;
    use std::fs;

    fn test_pipeline() -> ContextPipeline {
        ContextPipeline::with_indexer(LexicalWorkspaceIndexer::default_seeded())
    }

    fn init_token_budget(max_prompt_tokens: usize) {
        crate::settings::reset_for_test();
        let mut cfg = rex_config::RexConfig::defaults();
        cfg.context.max_prompt_tokens = max_prompt_tokens;
        crate::settings::init_for_test(Arc::new(rex_config::LoadedConfig::for_test(
            std::path::PathBuf::from("/tmp/rex-plugins-test"),
            cfg,
        )));
    }

    #[test]
    #[serial_test::serial]
    fn token_budget_from_config_overrides_defaults() {
        init_token_budget(64);
        let budget = TokenBudget::from_config();
        assert_eq!(budget.max_prompt_tokens, 64);
        crate::settings::reset_for_test();
    }

    #[test]
    fn build_injected_files_manifest_includes_active_file_and_readme() {
        let manifest = build_injected_files_manifest(
            Some("src/main.rs"),
            "[context]\n<<tool_result:fs.read>> README.md\n# rex",
            &["docs/AGENTS.md".to_string()],
        );
        assert_eq!(
            manifest,
            vec![
                "README.md".to_string(),
                "docs/AGENTS.md".to_string(),
                "src/main.rs".to_string()
            ]
        );
    }

    #[test]
    #[serial_test::serial]
    fn prompt_budget_truncates_when_config_limits_tokens() {
        init_token_budget(4);
        let mut pipeline = ContextPipeline::with_indexer(LexicalWorkspaceIndexer::default_seeded());
        let long = "a".repeat(200);
        let request = ContextRequest {
            prompt: long.clone(),
            diagnostics_hint: None,
            cache_bypass: true,
            behavior_snapshot: BehaviorSnapshot::default(),
            retrieve_off: true,
            active_file_path: None,
            harness_session_id: String::new(),
        };
        let result = pipeline.prepare(
            &request,
            AdapterCapabilities::for_runtime(RuntimeKind::Mock),
        );
        assert!(result.effective_prompt.chars().count() <= 16);
        crate::settings::reset_for_test();
    }

    #[test]
    #[serial_test::serial]
    fn advisory_intent_loads_manifest_bundle() {
        let dir = std::env::temp_dir().join(format!("rex-adv-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("docs")).expect("mkdir docs");
        fs::write(
            dir.join("docs/manifest.yaml"),
            "version: 1\ndocuments:\n  - path: docs/ROADMAP.md\n    role: explanation\n    status: active\n    advisory_bundle: true\n",
        )
        .expect("manifest");
        fs::write(dir.join("docs/ROADMAP.md"), "# Roadmap\n\nPriorities here.").expect("roadmap");
        let mut cfg = rex_config::RexConfig::defaults();
        cfg.context.advisory_intent_enabled = Some(true);
        let loaded = Arc::new(rex_config::LoadedConfig::for_test(dir.clone(), cfg));
        crate::settings::init_for_test(loaded);
        let prev = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&dir).expect("chdir");
        let (text, tokens, _) = load_advisory_bundle(512);
        std::env::set_current_dir(prev).expect("restore cwd");
        assert!(tokens > 0);
        assert!(text.contains("Priorities here"));
        crate::settings::reset_for_test();
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn workspace_indexer_reads_files_from_temp_root() {
        let dir = std::env::temp_dir().join(format!("rex-index-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).expect("mkdir");
        fs::write(dir.join("src/a.rs"), "socket retry logic").expect("write");
        fs::write(dir.join("README.md"), "project readme").expect("write");
        let index = LexicalWorkspaceIndexer::from_workspace_root(&dir);
        let hits = index.search("socket retry", 5);
        assert!(
            hits.iter().any(|c| c.source.contains("a.rs")),
            "expected workspace file in hits: {:?}",
            hits
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn extractive_compressor_prefers_query_overlapping_lines() {
        let compressor = ExtractiveContextCompressor;
        let chunks = vec![
            ContextChunk {
                source: "low.rs".to_string(),
                text: "unrelated module documentation".to_string(),
            },
            ContextChunk {
                source: "high.rs".to_string(),
                text: "socket retry daemon lifecycle".to_string(),
            },
        ];
        let (packed, _, _) = compressor.compress("socket retry", &chunks, 512);
        let high_pos = packed.find("high.rs").expect("high.rs line");
        let low_pos = packed.find("low.rs").expect("low.rs line");
        assert!(
            high_pos < low_pos,
            "expected query-relevant chunk first: {packed}"
        );
    }

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
    fn should_skip_retrieval_false_for_short_advisory_prompt() {
        let request = ContextRequest {
            prompt: "What should we do next?".to_string(),
            diagnostics_hint: None,
            cache_bypass: false,
            behavior_snapshot: BehaviorSnapshot::default(),
            retrieve_off: false,
            active_file_path: None,
            harness_session_id: String::new(),
        };
        assert!(!should_skip_retrieval(&request));
    }

    #[test]
    fn cache_key_uses_stable_prompt_prefix() {
        let a = stable_prefix_key("", "abcdef");
        let b = stable_prefix_key("", "abcdef and more");
        assert!(b.starts_with(&a));
    }

    #[test]
    fn prefix_cache_isolated_by_harness_session() {
        let mut pipeline = test_pipeline();
        let prompt = "status stream retry integration tests daemon lifecycle".to_string();
        let caps = AdapterCapabilities::for_runtime(RuntimeKind::Mock);
        let session_a = ContextRequest {
            prompt: prompt.clone(),
            diagnostics_hint: None,
            cache_bypass: false,
            behavior_snapshot: BehaviorSnapshot::default(),
            retrieve_off: false,
            active_file_path: None,
            harness_session_id: "session-a".to_string(),
        };
        let session_b = ContextRequest {
            prompt,
            diagnostics_hint: None,
            cache_bypass: false,
            behavior_snapshot: BehaviorSnapshot::default(),
            retrieve_off: false,
            active_file_path: None,
            harness_session_id: "session-b".to_string(),
        };
        assert_eq!(
            pipeline.prepare(&session_a, caps).metrics.cache_status,
            CacheStatus::MissStored
        );
        assert_eq!(
            pipeline.prepare(&session_b, caps).metrics.cache_status,
            CacheStatus::MissStored
        );
    }

    #[test]
    fn pipeline_reports_cache_hit_after_first_request() {
        let mut pipeline = test_pipeline();
        let request = ContextRequest {
            prompt: "status stream retry integration tests daemon lifecycle".to_string(),
            diagnostics_hint: None,
            cache_bypass: false,
            behavior_snapshot: BehaviorSnapshot::default(),
            retrieve_off: false,
            active_file_path: None,
            harness_session_id: String::new(),
        };
        let caps = AdapterCapabilities::for_runtime(RuntimeKind::Mock);
        let first = pipeline.prepare(&request, caps);
        assert_eq!(first.metrics.cache_status, CacheStatus::MissStored);
        let second = pipeline.prepare(&request, caps);
        assert_eq!(second.metrics.cache_status, CacheStatus::Hit);
    }

    #[test]
    fn pipeline_can_bypass_cache() {
        let mut pipeline = test_pipeline();
        let request = ContextRequest {
            prompt: "status stream retry integration tests daemon lifecycle".to_string(),
            diagnostics_hint: None,
            cache_bypass: true,
            behavior_snapshot: BehaviorSnapshot::default(),
            retrieve_off: false,
            active_file_path: None,
            harness_session_id: String::new(),
        };
        let result = pipeline.prepare(
            &request,
            AdapterCapabilities::for_runtime(RuntimeKind::Mock),
        );
        assert_eq!(result.metrics.cache_status, CacheStatus::Bypass);
    }

    #[test]
    fn mock_profile_attaches_context_when_indexer_hits() {
        let mut pipeline = test_pipeline();
        let request = ContextRequest {
            prompt: "status stream retry integration tests daemon lifecycle".to_string(),
            diagnostics_hint: None,
            cache_bypass: true,
            behavior_snapshot: BehaviorSnapshot::default(),
            retrieve_off: false,
            active_file_path: None,
            harness_session_id: String::new(),
        };
        let result = pipeline.prepare(
            &request,
            AdapterCapabilities::for_runtime(RuntimeKind::Mock),
        );
        assert!(result.effective_prompt.contains("[context]"));
    }

    #[test]
    fn cursor_profile_skips_context_attachment() {
        let mut pipeline = test_pipeline();
        let request = ContextRequest {
            prompt: "status stream retry integration tests daemon lifecycle".to_string(),
            diagnostics_hint: None,
            cache_bypass: true,
            behavior_snapshot: BehaviorSnapshot::default(),
            retrieve_off: false,
            active_file_path: None,
            harness_session_id: String::new(),
        };
        let result = pipeline.prepare(
            &request,
            AdapterCapabilities::for_runtime(RuntimeKind::CursorCli),
        );
        assert!(!result.effective_prompt.contains("[context]"));
    }

    #[test]
    fn retrieval_skipped_for_short_prompt() {
        let mut pipeline = test_pipeline();
        let request = ContextRequest {
            prompt: "hi".to_string(),
            diagnostics_hint: None,
            cache_bypass: true,
            behavior_snapshot: BehaviorSnapshot::default(),
            retrieve_off: false,
            active_file_path: None,
            harness_session_id: String::new(),
        };
        let result = pipeline.prepare(
            &request,
            AdapterCapabilities::for_runtime(RuntimeKind::Mock),
        );
        assert_eq!(result.metrics.retrieval, super::RetrievalDecision::Skipped);
        assert_eq!(result.metrics.context_candidates, 0);
    }

    #[test]
    fn retrieval_off_directive_skips_indexer() {
        let mut pipeline = test_pipeline();
        let request = ContextRequest {
            prompt: "status stream retry integration tests".to_string(),
            diagnostics_hint: None,
            cache_bypass: true,
            behavior_snapshot: BehaviorSnapshot::default(),
            retrieve_off: true,
            active_file_path: None,
            harness_session_id: String::new(),
        };
        let result = pipeline.prepare(
            &request,
            AdapterCapabilities::for_runtime(RuntimeKind::Mock),
        );
        assert_eq!(result.metrics.retrieval, super::RetrievalDecision::Skipped);
    }

    #[test]
    fn c1_strip_removes_extension_trailer_before_retrieval() {
        let mut pipeline = test_pipeline();
        let prompt = "status stream retry integration tests daemon lifecycle\n\n---\nFile: src/main.rs\nLanguage: rust".to_string();
        let request = ContextRequest {
            prompt,
            diagnostics_hint: None,
            cache_bypass: true,
            behavior_snapshot: BehaviorSnapshot::default(),
            retrieve_off: false,
            active_file_path: None,
            harness_session_id: String::new(),
        };
        let result = pipeline.prepare(
            &request,
            AdapterCapabilities::for_runtime(RuntimeKind::Mock),
        );
        assert!(result.c1_stripped);
        assert!(!result.effective_prompt.contains("File: src/main.rs"));
        assert_eq!(result.metrics.retrieval, super::RetrievalDecision::Ran);
    }

    #[test]
    fn behavior_filter_can_suppress_when_user_is_in_focus_flow() {
        let mut pipeline = test_pipeline();
        let request = ContextRequest {
            prompt: "complete this helper".to_string(),
            diagnostics_hint: None,
            cache_bypass: false,
            behavior_snapshot: BehaviorSnapshot {
                typing_cadence_cpm: 500,
                pause_events_last_minute: 0,
            },
            retrieve_off: false,
            active_file_path: None,
            harness_session_id: String::new(),
        };
        let result = pipeline.prepare(
            &request,
            AdapterCapabilities::for_runtime(RuntimeKind::Mock),
        );
        assert!(matches!(
            result.metrics.behavior_decision,
            super::BehaviorDecision::Suppress { .. }
        ));
    }
}

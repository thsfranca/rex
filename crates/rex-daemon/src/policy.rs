//! Daemon policy seam: routes a per-request `pipeline → cache decision → runtime`
//! flow through one explicit boundary so future cache layers (L2 semantic) and
//! orchestration (R008 approvals/checkpoints) plug in without re-touching
//! `service.rs`. See `docs/ARCHITECTURE_GUIDELINES.md` (R007 row) and
//! `docs/CACHING.md` for the policy this seam enforces.

use std::sync::Arc;

use rex_proto::rex::v1::StreamInferenceResponse;

use crate::adapters::RuntimeKind;
use crate::l1_cache::{L1Key, L1ResponseCache};

/// Inputs to a per-request cache/policy decision. Borrowed so the caller keeps
/// ownership of the prompt and mode strings.
#[derive(Debug, Clone)]
pub struct PolicyRequest<'a> {
    pub runtime: RuntimeKind,
    pub model: &'a str,
    pub mode: &'a str,
    pub effective_prompt: &'a str,
    pub cache_bypass: bool,
}

/// Reasons a request is not eligible for cache lookup or storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UncacheableReason {
    /// Modes other than `ask` are not cached today (see ADR 0003).
    NonAskMode,
}

/// What the daemon should do with response caching for one request. The decision
/// is computed once, before any cache I/O or runtime invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheDecision {
    /// Consult the cache with this key. On miss, run the runtime and store the
    /// result if it is cachable (`l1_cachable_responses`).
    Lookup(L1Key),
    /// Skip the cache entirely (operator/per-prompt bypass).
    Bypass,
    /// Mode is not eligible; just run the runtime, store nothing.
    Uncacheable { reason: UncacheableReason },
}

/// Resolve a request to its cache/policy decision. **Pure** — no I/O, no logging.
/// Service callers must take this decision before consulting any cache, which
/// preserves the `pipeline resolution → cache decision → runtime` ordering rule
/// from `docs/ARCHITECTURE_GUIDELINES.md`.
pub fn decide(req: &PolicyRequest<'_>) -> CacheDecision {
    if req.cache_bypass {
        return CacheDecision::Bypass;
    }
    match L1Key::try_new(
        req.runtime,
        req.model,
        req.mode,
        req.effective_prompt,
        false,
    ) {
        Some(key) => CacheDecision::Lookup(key),
        // `try_new` rejects only on bypass (handled above) or non-`ask` mode today.
        // Extend `UncacheableReason` if the rule set grows, rather than misclassify here.
        None => CacheDecision::Uncacheable {
            reason: UncacheableReason::NonAskMode,
        },
    }
}

/// Storage seam for response caches. L1 today; future L2 semantic cache plugs in
/// behind the same trait without changing `service.rs`.
pub trait ResponseCache: Send + Sync {
    fn get(&self, key: &L1Key) -> Option<Vec<StreamInferenceResponse>>;
    fn put(&self, key: L1Key, value: Vec<StreamInferenceResponse>);
}

/// Observable per-request cache outcome. Vocabulary matches `docs/CACHING.md`
/// "Metrics and observability" so daemon stdout, future dashboards, and the
/// architecture observability table share one set of labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheDecisionState {
    /// Cacheable lookup served from the response cache.
    Hit,
    /// Cacheable lookup missed; runtime ran (storage attempted when result eligible).
    MissStored,
    /// Operator bypass — cache neither read nor written.
    Bypass,
    /// Mode (or other policy) ineligible for caching today.
    UncacheableMode,
}

impl CacheDecisionState {
    /// Stable token used in daemon stdout, mirroring `l1_cache=` style. Intended
    /// for grep-based triage and future dashboards.
    pub fn label(self) -> &'static str {
        match self {
            Self::Hit => "hit",
            Self::MissStored => "miss_stored",
            Self::Bypass => "bypass",
            Self::UncacheableMode => "uncacheable_mode",
        }
    }

    /// Project a `CacheDecision` plus its lookup outcome onto the observable state.
    /// `hit` is ignored for `Bypass` / `Uncacheable` and only meaningful for `Lookup`.
    pub fn from_outcome(decision: &CacheDecision, hit: bool) -> Self {
        match decision {
            CacheDecision::Lookup(_) if hit => Self::Hit,
            CacheDecision::Lookup(_) => Self::MissStored,
            CacheDecision::Bypass => Self::Bypass,
            CacheDecision::Uncacheable {
                reason: UncacheableReason::NonAskMode,
            } => Self::UncacheableMode,
        }
    }
}

/// Holds the daemon's response cache behind one boundary so the service layer
/// does not depend on a concrete cache type. Constructed by `RexDaemonService`
/// at startup; tests can substitute any `ResponseCache` impl.
pub struct PolicyEngine {
    cache: Arc<dyn ResponseCache>,
}

impl PolicyEngine {
    pub fn new(cache: Arc<dyn ResponseCache>) -> Self {
        Self { cache }
    }

    /// Default engine using the in-process L1 LRU cache.
    pub fn with_default_l1() -> Self {
        Self::new(Arc::new(L1ResponseCache::new()))
    }

    pub fn decide(&self, req: &PolicyRequest<'_>) -> CacheDecision {
        decide(req)
    }

    pub fn get(&self, key: &L1Key) -> Option<Vec<StreamInferenceResponse>> {
        self.cache.get(key)
    }

    pub fn put(&self, key: L1Key, value: Vec<StreamInferenceResponse>) {
        self.cache.put(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn req<'a>(runtime: RuntimeKind, mode: &'a str, cache_bypass: bool) -> PolicyRequest<'a> {
        PolicyRequest {
            runtime,
            model: "",
            mode,
            effective_prompt: "ping",
            cache_bypass,
        }
    }

    #[test]
    fn ask_mode_resolves_to_lookup_for_each_runtime() {
        for runtime in [RuntimeKind::Mock, RuntimeKind::CursorCli] {
            let decision = decide(&req(runtime, "ask", false));
            match decision {
                CacheDecision::Lookup(_) => {}
                other => panic!("expected Lookup for ask/{runtime:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn bypass_overrides_mode() {
        for mode in ["ask", "plan", "agent", "AGENT", " ask "] {
            assert_eq!(
                decide(&req(RuntimeKind::Mock, mode, true)),
                CacheDecision::Bypass,
                "bypass should win regardless of mode={mode}"
            );
        }
    }

    #[test]
    fn non_ask_modes_are_uncacheable() {
        for mode in ["plan", "agent", "PLAN", "  AGENT  ", "unknown"] {
            assert_eq!(
                decide(&req(RuntimeKind::Mock, mode, false)),
                CacheDecision::Uncacheable {
                    reason: UncacheableReason::NonAskMode,
                },
                "expected NonAskMode for mode={mode}"
            );
        }
    }

    #[test]
    fn empty_mode_normalizes_to_ask_and_caches() {
        assert!(matches!(
            decide(&req(RuntimeKind::Mock, "", false)),
            CacheDecision::Lookup(_)
        ));
    }

    /// Records the order of cache calls to prove the service layer cannot
    /// consult the cache before `decide` produces a key.
    #[derive(Default)]
    struct RecordingCache {
        events: Mutex<Vec<String>>,
    }

    impl ResponseCache for RecordingCache {
        fn get(&self, key: &L1Key) -> Option<Vec<StreamInferenceResponse>> {
            self.events
                .lock()
                .expect("recording cache mutex")
                .push(format!("get:{}:{}", key.adapter, key.mode));
            None
        }

        fn put(&self, key: L1Key, _value: Vec<StreamInferenceResponse>) {
            self.events
                .lock()
                .expect("recording cache mutex")
                .push(format!("put:{}:{}", key.adapter, key.mode));
        }
    }

    #[test]
    fn decide_does_not_touch_cache() {
        let recorder = Arc::new(RecordingCache::default());
        let engine = PolicyEngine::new(recorder.clone());
        let _ = engine.decide(&req(RuntimeKind::Mock, "ask", false));
        let _ = engine.decide(&req(RuntimeKind::Mock, "agent", false));
        let _ = engine.decide(&req(RuntimeKind::Mock, "ask", true));
        assert!(
            recorder.events.lock().expect("mutex").is_empty(),
            "decide must be pure: no cache I/O permitted"
        );
    }

    #[test]
    fn cache_decision_state_labels_match_docs_caching() {
        assert_eq!(CacheDecisionState::Hit.label(), "hit");
        assert_eq!(CacheDecisionState::MissStored.label(), "miss_stored");
        assert_eq!(CacheDecisionState::Bypass.label(), "bypass");
        assert_eq!(
            CacheDecisionState::UncacheableMode.label(),
            "uncacheable_mode"
        );
    }

    #[test]
    fn cache_decision_state_maps_each_decision_branch() {
        let lookup_key = match decide(&req(RuntimeKind::Mock, "ask", false)) {
            CacheDecision::Lookup(k) => k,
            other => panic!("expected Lookup, got {other:?}"),
        };
        let lookup = CacheDecision::Lookup(lookup_key);
        assert_eq!(
            CacheDecisionState::from_outcome(&lookup, true),
            CacheDecisionState::Hit
        );
        assert_eq!(
            CacheDecisionState::from_outcome(&lookup, false),
            CacheDecisionState::MissStored
        );
        assert_eq!(
            CacheDecisionState::from_outcome(&CacheDecision::Bypass, false),
            CacheDecisionState::Bypass
        );
        assert_eq!(
            CacheDecisionState::from_outcome(
                &CacheDecision::Uncacheable {
                    reason: UncacheableReason::NonAskMode,
                },
                false,
            ),
            CacheDecisionState::UncacheableMode
        );
    }

    #[test]
    fn engine_routes_get_put_to_underlying_cache() {
        let recorder = Arc::new(RecordingCache::default());
        let engine = PolicyEngine::new(recorder.clone());
        let key = match engine.decide(&req(RuntimeKind::Mock, "ask", false)) {
            CacheDecision::Lookup(k) => k,
            other => panic!("expected Lookup, got {other:?}"),
        };
        assert!(engine.get(&key).is_none());
        engine.put(key.clone(), vec![]);
        let events = recorder.events.lock().expect("mutex").clone();
        assert_eq!(
            events,
            vec!["get:mock:ask".to_string(), "put:mock:ask".to_string()]
        );
    }
}

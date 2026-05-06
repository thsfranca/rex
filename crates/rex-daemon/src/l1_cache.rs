use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::Mutex;

use lru::LruCache;
use rex_proto::rex::v1::StreamInferenceResponse;
use tonic::Status;

use crate::adapters::RuntimeKind;
use crate::domain::ACTIVE_MODEL_ID;

/// Bumped when the L1 key shape or response semantic changes (invalidates on-disk or cross-version hits).
pub const L1_CACHE_SCHEMA: u32 = 1;

const L1_MAX_ENTRIES: usize = 256;

/// Normalized (empty → `ask`); trim + lowercase ASCII.
pub fn normalize_mode(raw: &str) -> String {
    let t = raw.trim();
    if t.is_empty() {
        "ask".to_string()
    } else {
        t.to_ascii_lowercase()
    }
}

/// Collapses line endings to `\n` and removes trailing space per line, then trim.
fn normalize_prompt_for_l1(s: &str) -> String {
    s.replace("\r\n", "\n")
        .split('\n')
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn hash_effective_prompt(s: &str) -> u64 {
    let normalized = normalize_prompt_for_l1(s);
    let mut h = DefaultHasher::new();
    normalized.hash(&mut h);
    h.finish()
}

fn workspace_fingerprint() -> String {
    std::env::var("REX_WORKSPACE_ROOT")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "default".to_string())
}

/// Key for the L1 **exact** response cache (in-memory, process-local).
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct L1Key {
    pub adapter: String,
    pub model: String,
    pub mode: String,
    pub prompt_hash: u64,
    pub schema: u32,
    pub workspace: String,
}

impl L1Key {
    pub fn try_new(
        runtime: RuntimeKind,
        model: &str,
        mode: &str,
        effective_prompt: &str,
        cache_bypass: bool,
    ) -> Option<Self> {
        if cache_bypass {
            return None;
        }
        let mode_n = normalize_mode(mode);
        if mode_n != "ask" {
            return None;
        }
        let model_id = {
            let t = model.trim();
            if t.is_empty() {
                ACTIVE_MODEL_ID.to_string()
            } else {
                t.to_string()
            }
        };
        let adapter = match runtime {
            RuntimeKind::Mock => "mock".to_string(),
            RuntimeKind::CursorCli => "cursor-cli".to_string(),
        };
        Some(L1Key {
            adapter,
            model: model_id,
            mode: mode_n,
            prompt_hash: hash_effective_prompt(effective_prompt),
            schema: L1_CACHE_SCHEMA,
            workspace: workspace_fingerprint(),
        })
    }
}

/// Returns `true` if we may keep this stream result for a future L1 read.
pub fn l1_cachable_responses(
    items: &[Result<StreamInferenceResponse, Status>],
) -> Option<Vec<StreamInferenceResponse>> {
    if items.is_empty() {
        return None;
    }
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let chunk = item.as_ref().ok()?;
        out.push(chunk.clone());
    }
    let last = out.last()?;
    if !last.done {
        return None;
    }
    Some(out)
}

pub struct L1ResponseCache {
    inner: Mutex<LruCache<L1Key, Vec<StreamInferenceResponse>>>,
}

impl L1ResponseCache {
    pub fn new() -> Self {
        let cap = NonZeroUsize::new(L1_MAX_ENTRIES).expect("non-zero l1 cap");
        Self {
            inner: Mutex::new(LruCache::new(cap)),
        }
    }

    pub fn get(&self, key: &L1Key) -> Option<Vec<StreamInferenceResponse>> {
        self.inner.lock().expect("l1 cache mutex").get(key).cloned()
    }

    pub fn put(&self, key: L1Key, value: Vec<StreamInferenceResponse>) {
        self.inner.lock().expect("l1 cache mutex").put(key, value);
    }
}

impl Default for L1ResponseCache {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::policy::ResponseCache for L1ResponseCache {
    fn get(&self, key: &L1Key) -> Option<Vec<StreamInferenceResponse>> {
        L1ResponseCache::get(self, key)
    }

    fn put(&self, key: L1Key, value: Vec<StreamInferenceResponse>) {
        L1ResponseCache::put(self, key, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_mode_defaults_empty_to_ask() {
        assert_eq!(normalize_mode(""), "ask");
        assert_eq!(normalize_mode("  ASK  "), "ask");
    }

    #[test]
    fn ask_mode_creates_key() {
        let k = L1Key::try_new(RuntimeKind::Mock, "", "ask", "hello", false)
            .expect("ask should cache under L1");
        assert_eq!(k.model, ACTIVE_MODEL_ID);
        assert_eq!(k.mode, "ask");
    }

    #[test]
    fn agent_mode_skips_l1() {
        assert!(L1Key::try_new(RuntimeKind::Mock, "", "agent", "x", false).is_none());
    }

    #[test]
    fn cache_bypass_skips() {
        assert!(L1Key::try_new(RuntimeKind::Mock, "", "ask", "x", true).is_none());
    }

    #[test]
    fn cachable_responses_require_done() {
        let a = vec![Ok(StreamInferenceResponse {
            text: "a".to_string(),
            index: 0,
            done: false,
        })];
        assert!(l1_cachable_responses(&a).is_none());
    }

    #[test]
    fn l1_put_get_round_trip() {
        let cache = L1ResponseCache::new();
        let key = L1Key::try_new(RuntimeKind::Mock, "", "ask", "ping", false).expect("key");
        let v = vec![
            StreamInferenceResponse {
                text: "x".to_string(),
                index: 0,
                done: false,
            },
            StreamInferenceResponse {
                text: String::new(),
                index: 1,
                done: true,
            },
        ];
        cache.put(key.clone(), v.clone());
        assert_eq!(cache.get(&key), Some(v));
    }
}

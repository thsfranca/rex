//! Per-tool approval tokens for broker checkpoint gating (R073d).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const APPROVAL_TTL: Duration = Duration::from_secs(300);

struct PendingToolApproval {
    capability: String,
    path: String,
    tool_call_id: String,
    created: Instant,
    approved: Option<bool>,
}

static STORE: OnceLock<Mutex<HashMap<String, PendingToolApproval>>> = OnceLock::new();

fn store() -> &'static Mutex<HashMap<String, PendingToolApproval>> {
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn prune_expired(map: &mut HashMap<String, PendingToolApproval>) {
    map.retain(|_, entry| entry.created.elapsed() < APPROVAL_TTL);
}

/// Register a pending tool approval; returns token for client response.
pub fn register_pending(
    capability: &str,
    path: &str,
    tool_call_id: &str,
) -> String {
    let token = format!(
        "tap-{}-{}",
        std::process::id(),
        Instant::now().elapsed().as_nanos()
    );
    let mut map = store().lock().expect("tool approval store");
    prune_expired(&mut map);
    map.insert(
        token.clone(),
        PendingToolApproval {
            capability: capability.to_string(),
            path: path.to_string(),
            tool_call_id: tool_call_id.to_string(),
            created: Instant::now(),
            approved: None,
        },
    );
    token
}

/// Returns `Some(true|false)` when decided; `None` if unknown/expired token.
pub fn respond(token: &str, approved: bool) -> Option<bool> {
    let mut map = store().lock().expect("tool approval store");
    prune_expired(&mut map);
    let entry = map.get_mut(token)?;
    entry.approved = Some(approved);
    Some(approved)
}

/// Whether `token` was approved by the operator.
pub fn is_approved(token: &str) -> bool {
    let map = store().lock().expect("tool approval store");
    map.get(token)
        .and_then(|e| e.approved)
        .is_some_and(|v| v)
}

pub fn approval_required_error(token: &str) -> String {
    format!("approval_required:{token}")
}

pub fn is_approval_required_error(error: &str) -> bool {
    error.starts_with("approval_required:")
}

pub fn token_from_error(error: &str) -> Option<&str> {
    error.strip_prefix("approval_required:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_respond_approval() {
        let token = register_pending("fs.write", "a.txt", "call-1");
        assert!(!is_approved(&token));
        assert_eq!(respond(&token, true), Some(true));
        assert!(is_approved(&token));
    }
}

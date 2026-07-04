//! Per-terminal harness session identity (parallel harness isolation).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub const METADATA_KEY: &str = "x-rex-harness-session-id";

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Stable id for one bare-`rex` TUI process; sent on every `StreamInference` call.
///
/// When `REX_HARNESS_SESSION_ID` is set (probe/fixture harness only), that value is used
/// so tuiwright baselines stay stable across runs.
pub fn new_harness_session_id() -> String {
    if let Ok(id) = std::env::var("REX_HARNESS_SESSION_ID") {
        let trimmed = id.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    let seq = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("hs-{}-{}-{}", std::process::id(), seq, nanos)
}

pub fn insert_metadata(
    metadata: &mut tonic::metadata::MetadataMap,
    session_id: &str,
) -> Result<(), tonic::Status> {
    let value = tonic::metadata::MetadataValue::try_from(session_id)
        .map_err(|_| tonic::Status::invalid_argument("invalid harness session id"))?;
    metadata.insert(METADATA_KEY, value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_override_pins_probe_session() {
        std::env::set_var("REX_HARNESS_SESSION_ID", "hs-probe");
        assert_eq!(new_harness_session_id(), "hs-probe");
        std::env::remove_var("REX_HARNESS_SESSION_ID");
    }
}

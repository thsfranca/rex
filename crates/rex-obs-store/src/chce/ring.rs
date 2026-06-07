use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::sync::Arc;
use std::time::Duration;

use crate::record::StreamEconomicsRecord;

/// One stream append accepted on the hot path.
#[derive(Debug, Clone)]
pub struct StreamAppendEvent {
    pub record: StreamEconomicsRecord,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingPushError {
    Full,
}

/// Bounded ingest queue for non-blocking `append_stream` dispatch.
pub struct LiveRingBuffer {
    tx: SyncSender<StreamAppendEvent>,
    rx: Option<mpsc::Receiver<StreamAppendEvent>>,
    pending: Arc<AtomicUsize>,
}

pub struct RingReceiver {
    inner: mpsc::Receiver<StreamAppendEvent>,
    pending: Arc<AtomicUsize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingRecvError {
    Timeout,
    Disconnected,
}

impl LiveRingBuffer {
    pub fn new(capacity: usize) -> Self {
        let (tx, rx) = mpsc::sync_channel(capacity.max(1));
        Self {
            tx,
            rx: Some(rx),
            pending: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn take_receiver(&mut self) -> RingReceiver {
        let inner = self
            .rx
            .take()
            .expect("LiveRingBuffer receiver already taken");
        RingReceiver {
            inner,
            pending: Arc::clone(&self.pending),
        }
    }

    pub fn push(&self, event: StreamAppendEvent) -> Result<(), RingPushError> {
        self.pending.fetch_add(1, Ordering::AcqRel);
        match self.tx.try_send(event) {
            Ok(()) => Ok(()),
            Err(err) => {
                self.pending.fetch_sub(1, Ordering::AcqRel);
                Err(match err {
                    TrySendError::Full(_) => RingPushError::Full,
                    TrySendError::Disconnected(_) => RingPushError::Full,
                })
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.pending.load(Ordering::Acquire) == 0
    }
}

impl RingReceiver {
    pub fn recv_timeout(&self, timeout: Duration) -> Result<StreamAppendEvent, RingRecvError> {
        match self.inner.recv_timeout(timeout) {
            Ok(event) => {
                self.pending.fetch_sub(1, Ordering::AcqRel);
                Ok(event)
            }
            Err(mpsc::RecvTimeoutError::Timeout) => Err(RingRecvError::Timeout),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(RingRecvError::Disconnected),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event(request_id: u64) -> StreamAppendEvent {
        StreamAppendEvent {
            record: StreamEconomicsRecord {
                snapshot_id: "snap".to_string(),
                request_id,
                trace_id: format!("trace-{request_id}"),
                turn_id: "turn-1".to_string(),
                terminal: "done".to_string(),
                route: "sidecar+mock".to_string(),
                cache_decision: "miss_stored".to_string(),
                decision_id: format!("dec-{request_id}"),
                inference_runtime: "mock".to_string(),
                mode: "ask".to_string(),
                model: "gpt-4o-mini".to_string(),
                elapsed_ms: 42,
                chunks_sent: 3,
                prompt_tokens: 100,
                context_tokens: 50,
                context_candidates: 10,
                context_selected: 5,
                context_truncated: false,
                retrieval: "skipped".to_string(),
                compression_strategy: "extractive_query".to_string(),
                cached_tokens: None,
                prefix_hash: None,
                parse_retries: None,
            },
            created_at_ms: 1_700_000_000_000,
        }
    }

    #[test]
    fn push_is_non_blocking_until_full() {
        let mut ring = LiveRingBuffer::new(2);
        let rx = ring.take_receiver();
        assert!(ring.push(sample_event(1)).is_ok());
        assert!(ring.push(sample_event(2)).is_ok());
        assert_eq!(ring.push(sample_event(3)), Err(RingPushError::Full));
        assert_eq!(rx.inner.try_recv().unwrap().record.request_id, 1);
    }
}

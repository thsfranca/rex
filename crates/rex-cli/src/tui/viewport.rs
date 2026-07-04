//! In-memory transcript viewport (daemon is SoT; ADR 0040).

use rex_proto::rex::v1::SessionEvent;

use super::state::{TranscriptMessage, TranscriptRole};

pub const DEFAULT_FETCH_LIMIT: u32 = 30;

#[derive(Debug, Clone, Default)]
pub struct ViewportCache {
    pub oldest_loaded_sequence: u64,
    pub newest_sequence: u64,
    pub head_sequence: u64,
    pub has_more_before: bool,
    pub fetching_history: bool,
    /// Older turns prepended from retroactive fetch (hot tail stays in `messages`).
    pub prefetch_older: Vec<TranscriptMessage>,
}

impl ViewportCache {
    pub fn begin_history_fetch(&mut self) {
        self.fetching_history = true;
    }

    pub fn end_history_fetch(&mut self) {
        self.fetching_history = false;
    }

    pub fn retroactive_cursor(&self) -> u64 {
        if self.oldest_loaded_sequence > 0 {
            self.oldest_loaded_sequence
        } else {
            self.head_sequence.saturating_add(1)
        }
    }

    pub fn merge_retroactive(
        &mut self,
        events: Vec<SessionEvent>,
        has_more_before: bool,
        head_sequence: u64,
    ) -> Vec<TranscriptMessage> {
        self.head_sequence = head_sequence;
        self.has_more_before = has_more_before;
        if events.is_empty() {
            return Vec::new();
        }
        let mut older = events_to_messages(&events);
        if let Some(first) = events.first() {
            self.oldest_loaded_sequence = first.sequence;
        }
        if self.prefetch_older.is_empty() {
            self.prefetch_older = older;
        } else {
            older.append(&mut self.prefetch_older);
            self.prefetch_older = older;
        }
        self.prefetch_older.clone()
    }

    pub fn apply_incremental(
        &mut self,
        events: Vec<SessionEvent>,
        has_more_after: bool,
        head_sequence: u64,
    ) {
        self.head_sequence = head_sequence;
        if events.is_empty() {
            self.has_more_before = self.has_more_before || has_more_after;
            return;
        }
        if self.oldest_loaded_sequence == 0 || events.first().map(|e| e.sequence) < Some(self.oldest_loaded_sequence) {
            self.oldest_loaded_sequence = events.first().map(|e| e.sequence).unwrap_or(0);
        }
        self.newest_sequence = events.last().map(|e| e.sequence).unwrap_or(self.newest_sequence);
        self.has_more_before = self.has_more_before || has_more_after;
    }
}

pub fn events_to_messages(events: &[SessionEvent]) -> Vec<TranscriptMessage> {
    let mut out = Vec::new();
    let mut agent_buf = String::new();
    for event in events {
        match event.event.as_str() {
            "operator_prompt" => {
                flush_agent(&mut out, &mut agent_buf);
                if !event.text.trim().is_empty() {
                    out.push(TranscriptMessage {
                        role: TranscriptRole::Operator,
                        body: event.text.clone(),
                    });
                }
            }
            "chunk" if !event.text.is_empty() => {
                agent_buf.push_str(&event.text);
            }
            "done" => flush_agent(&mut out, &mut agent_buf),
            _ => {}
        }
    }
    flush_agent(&mut out, &mut agent_buf);
    out
}

fn flush_agent(out: &mut Vec<TranscriptMessage>, buf: &mut String) {
    let body = buf.trim().to_string();
    if !body.is_empty() {
        out.push(TranscriptMessage {
            role: TranscriptRole::Agent,
            body,
        });
    }
    buf.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(seq: u64, event: &str, text: &str) -> SessionEvent {
        SessionEvent {
            sequence: seq,
            event: event.to_string(),
            text: text.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn retroactive_merge_prepends_without_dropping_tail_marker() {
        let mut cache = ViewportCache::default();
        cache.newest_sequence = 5;
        cache.head_sequence = 5;
        let older = cache.merge_retroactive(
            vec![ev(1, "operator_prompt", "old"), ev(2, "chunk", "reply"), ev(3, "done", "")],
            true,
            5,
        );
        assert_eq!(older.len(), 2);
        assert_eq!(cache.oldest_loaded_sequence, 1);
        assert!(cache.has_more_before);
        assert_eq!(cache.prefetch_older.len(), 2);
    }

    #[test]
    fn events_to_messages_groups_chunks_until_done() {
        let msgs = events_to_messages(&[
            ev(1, "operator_prompt", "hi"),
            ev(2, "chunk", "hel"),
            ev(3, "chunk", "lo"),
            ev(4, "done", ""),
        ]);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[1].body, "hello");
    }
}

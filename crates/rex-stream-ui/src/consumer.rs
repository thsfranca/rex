use std::collections::HashSet;

use crate::event::{parse_stream_line, StreamEvent};
use crate::turn_state::{TurnState, UiEffect};

/// NDJSON stream consumer with turn cancellation and state projection.
#[derive(Debug, Default)]
pub struct StreamConsumer {
    pub state: TurnState,
    canceled_turn_ids: HashSet<String>,
    active_turn_id: Option<String>,
}

impl StreamConsumer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a turn as canceled; subsequent events with this `turn_id` are ignored.
    pub fn cancel_turn(&mut self, turn_id: impl Into<String>) {
        self.canceled_turn_ids.insert(turn_id.into());
    }

    pub fn clear_canceled_turns(&mut self) {
        self.canceled_turn_ids.clear();
    }

    pub fn set_active_turn_id(&mut self, turn_id: Option<String>) {
        self.active_turn_id = turn_id;
    }

    /// Parse and apply one NDJSON line. Returns effects (empty when ignored).
    pub fn feed_line(&mut self, line: &str) -> Result<Vec<UiEffect>, String> {
        let event = parse_stream_line(line)?;
        Ok(self.feed_event(event))
    }

    /// Apply a parsed stream event.
    pub fn feed_event(&mut self, event: StreamEvent) -> Vec<UiEffect> {
        if let Some(turn_id) = event.turn_id() {
            if self.canceled_turn_ids.contains(turn_id) {
                return vec![UiEffect::Ignored];
            }
            if self.active_turn_id.is_none() {
                self.active_turn_id = Some(turn_id.to_string());
            }
        }

        match &event {
            StreamEvent::Done { .. } | StreamEvent::Error { .. } => {
                self.active_turn_id = None;
            }
            _ => {}
        }

        self.state.apply(&event)
    }

    pub fn begin_turn(&mut self, turn_id: Option<String>) {
        self.state.reset_to_idle();
        self.active_turn_id = turn_id;
    }

    /// Apply a gRPC stream chunk when the `grpc` feature is enabled.
    #[cfg(feature = "grpc")]
    pub fn feed_grpc_chunk(
        &mut self,
        chunk: &rex_proto::rex::v1::StreamInferenceResponse,
    ) -> Vec<UiEffect> {
        match crate::grpc::stream_event_from_grpc(chunk) {
            Some(event) => self.feed_event(event),
            None => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::turn_state::UiEffect;

    #[test]
    fn canceled_turn_events_are_ignored() {
        let mut consumer = StreamConsumer::new();
        consumer.cancel_turn("turn-1");
        let effects = consumer
            .feed_line(
                r#"{"event":"chunk","index":0,"text":"x","turn_id":"turn-1"}"#,
            )
            .unwrap();
        assert!(matches!(effects.as_slice(), [UiEffect::Ignored]));
        assert!(consumer.state.output_text.is_empty());
    }

    #[test]
    fn feeds_happy_path_fixture() {
        let fixture = include_str!("../../../fixtures/ndjson_contract/happy_path.ndjson");
        let mut consumer = StreamConsumer::new();
        let mut terminal = false;
        for line in fixture.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let effects = consumer.feed_line(line).unwrap();
            if effects
                .iter()
                .any(|e| matches!(e, UiEffect::TerminalDone))
            {
                terminal = true;
            }
        }
        assert!(terminal);
        assert_eq!(consumer.state.output_text, "hello world");
    }
}

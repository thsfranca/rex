//! Convert gRPC stream chunks to [`StreamEvent`] (optional `grpc` feature).

use crate::event::StreamEvent;

/// Map a daemon `StreamInferenceResponse` chunk to a harness stream event.
pub fn stream_event_from_grpc(
    chunk: &rex_proto::rex::v1::StreamInferenceResponse,
) -> Option<StreamEvent> {
    if chunk.done {
        return Some(StreamEvent::Done { index: chunk.index });
    }

    let event = chunk.event.trim();
    match event {
        "" | "chunk" => {
            if chunk.text.is_empty() {
                None
            } else {
                Some(StreamEvent::Chunk {
                    index: chunk.index,
                    text: chunk.text.clone(),
                    turn_id: optional_string(&chunk.turn_id),
                    sequence: optional_u64(chunk.sequence),
                })
            }
        }
        "tool" => Some(StreamEvent::Tool {
            index: chunk.index,
            name: chunk.tool_name.clone(),
            phase: chunk.phase.clone(),
            detail: chunk.detail.clone(),
            tool_call_id: optional_string(&chunk.tool_call_id),
            turn_id: optional_string(&chunk.turn_id),
            sequence: optional_u64(chunk.sequence),
            elapsed_ms: optional_u64(chunk.elapsed_ms),
        }),
        "step" => Some(StreamEvent::Step {
            index: chunk.index,
            phase: chunk.phase.clone(),
            summary: chunk.summary.clone(),
            turn_id: optional_string(&chunk.turn_id),
            sequence: optional_u64(chunk.sequence),
        }),
        "plan" => Some(StreamEvent::Plan {
            index: chunk.index,
            phase: chunk.phase.clone(),
            title: chunk.summary.clone(),
            detail: chunk.detail.clone(),
            sequence: optional_u64(chunk.sequence),
        }),
        "activity" => Some(StreamEvent::Activity {
            index: chunk.index,
            phase: chunk.phase.clone(),
            summary: chunk.summary.clone(),
            detail: chunk.detail.clone(),
            sequence: optional_u64(chunk.sequence),
        }),
        "error" => Some(StreamEvent::Error {
            message: if chunk.text.is_empty() {
                chunk.summary.clone()
            } else {
                chunk.text.clone()
            },
            code: if chunk.phase.is_empty() {
                "stream_error".to_string()
            } else {
                chunk.phase.clone()
            },
        }),
        _ => None,
    }
}

fn optional_string(value: &str) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn optional_u64(value: u64) -> Option<u64> {
    if value > 0 {
        Some(value)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rex_proto::rex::v1::StreamInferenceResponse;

    #[test]
    fn maps_chunk_response() {
        let chunk = StreamInferenceResponse {
            text: "hi".to_string(),
            index: 1,
            done: false,
            ..Default::default()
        };
        let event = stream_event_from_grpc(&chunk).unwrap();
        assert!(matches!(event, StreamEvent::Chunk { text, .. } if text == "hi"));
    }

    #[test]
    fn maps_error_response() {
        let chunk = StreamInferenceResponse {
            text: "boom".to_string(),
            index: 2,
            done: false,
            event: "error".to_string(),
            phase: "mock_error".to_string(),
            ..Default::default()
        };
        let event = stream_event_from_grpc(&chunk).unwrap();
        assert!(
            matches!(event, StreamEvent::Error { message, code } if message == "boom" && code == "mock_error")
        );
    }
}

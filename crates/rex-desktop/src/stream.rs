use rex_cli::transport::connect_client;
use rex_cli::{ensure_daemon_ready, CliError};
use rex_proto::rex::v1::StreamInferenceRequest;
use rex_stream_ui::{StreamConsumer, TurnPhase, UiEffect};
use serde::Serialize;
use tauri::ipc::Channel;
use tonic::Streaming;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StreamEventDto {
    Chunk { text: String },
    Phase { phase: String },
    Message { text: String },
    ApprovalRequired {
        #[serde(rename = "toolCallId")]
        tool_call_id: String,
        #[serde(rename = "toolName")]
        tool_name: String,
        detail: String,
        #[serde(rename = "approvalToken")]
        approval_token: String,
    },
    Done,
    Error { code: String, message: String },
}

pub async fn submit_prompt_stream(
    prompt: String,
    mode: String,
    trace_id: String,
    harness_session_id: String,
    channel: Channel<StreamEventDto>,
) -> Result<(), CliError> {
    ensure_daemon_ready().await?;
    let mut client = connect_client(Some(&trace_id)).await?;
    let mut request = tonic::Request::new(StreamInferenceRequest {
        prompt,
        model: String::new(),
        mode,
        approval_id: String::new(),
        client_hints: None,
        continue_token: String::new(),
    });
    let metadata_value = tonic::metadata::MetadataValue::try_from(trace_id.as_str())
        .map_err(|_| CliError::Status(tonic::Status::invalid_argument("invalid trace id")))?;
    request
        .metadata_mut()
        .insert("x-rex-trace-id", metadata_value);
    rex_cli::insert_harness_session_metadata(request.metadata_mut(), &harness_session_id)
        .map_err(CliError::Status)?;

    let response = client.stream_inference(request).await?;
    let mut stream: Streaming<rex_proto::rex::v1::StreamInferenceResponse> = response.into_inner();
    let mut consumer = StreamConsumer::new();

    loop {
        let chunk = match stream.message().await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => break,
            Err(status) => {
                if consumer.state.phase == TurnPhase::ToolApproval {
                    return Ok(());
                }
                return Err(CliError::Status(status));
            }
        };
        if chunk.done {
            let _ = channel.send(StreamEventDto::Done);
            return Ok(());
        }
        for effect in consumer.feed_grpc_chunk(&chunk) {
            if let Some(dto) = map_effect(effect) {
                let _ = channel.send(dto);
            }
        }
    }
    if consumer.state.phase == TurnPhase::ToolApproval {
        return Ok(());
    }
    let _ = channel.send(StreamEventDto::Error {
        code: "stream_incomplete".to_string(),
        message: "Stream ended without done".to_string(),
    });
    Ok(())
}

fn map_effect(effect: UiEffect) -> Option<StreamEventDto> {
    match effect {
        UiEffect::AppendChunk(text) => Some(StreamEventDto::Chunk { text }),
        UiEffect::OperatorMessage(text) => Some(StreamEventDto::Message { text }),
        UiEffect::PhaseChanged(phase) => Some(StreamEventDto::Phase {
            phase: phase_to_str(phase).to_string(),
        }),
        UiEffect::ToolUpdated(card) if card.phase == "approval_required" => {
            let (detail, approval_token) = split_approval_detail(&card.detail);
            Some(StreamEventDto::ApprovalRequired {
                tool_call_id: card.tool_call_id,
                tool_name: card.name,
                detail,
                approval_token,
            })
        }
        UiEffect::TerminalDone => Some(StreamEventDto::Done),
        UiEffect::TerminalError { code, message } => {
            if let Some(token) = message.strip_prefix("approval_required:") {
                return Some(StreamEventDto::ApprovalRequired {
                    tool_call_id: String::new(),
                    tool_name: String::new(),
                    detail: String::new(),
                    approval_token: token.to_string(),
                });
            }
            Some(StreamEventDto::Error { code, message })
        }
        UiEffect::ToolUpdated(_)
        | UiEffect::Ignored => None,
    }
}

fn phase_to_str(phase: TurnPhase) -> &'static str {
    match phase {
        TurnPhase::Idle => "idle",
        TurnPhase::Generating => "generating",
        TurnPhase::ToolRunning => "tool_running",
        TurnPhase::ToolApproval => "tool_approval",
        TurnPhase::Terminal => "terminal",
    }
}

fn split_approval_detail(detail: &str) -> (String, String) {
    if let Some((path, token)) = detail.rsplit_once('|') {
        if token.starts_with("tap-") {
            return (path.to_string(), token.to_string());
        }
    }
    (detail.to_string(), String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approval_required_serializes_camel_case_fields() {
        let dto = StreamEventDto::ApprovalRequired {
            tool_call_id: "tc1".into(),
            tool_name: "fs.write".into(),
            detail: "path".into(),
            approval_token: "tap-1".into(),
        };
        let value: serde_json::Value = serde_json::to_value(&dto).unwrap();
        assert_eq!(value["kind"], "approvalRequired");
        assert_eq!(value["toolCallId"], "tc1");
        assert_eq!(value["toolName"], "fs.write");
        assert_eq!(value["approvalToken"], "tap-1");
    }
}

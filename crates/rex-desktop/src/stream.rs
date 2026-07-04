use rex_cli::transport::connect_client;
use rex_cli::CliError;
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

    while let Some(chunk) = stream.message().await.map_err(CliError::Status)? {
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
        UiEffect::TerminalDone => Some(StreamEventDto::Done),
        UiEffect::TerminalError { code, message } => Some(StreamEventDto::Error { code, message }),
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

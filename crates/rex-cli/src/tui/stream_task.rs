//! Background stream consumer task.

use rex_proto::rex::v1::StreamInferenceRequest;
use rex_stream_ui::{StreamConsumer, UiEffect};
use tokio::sync::mpsc;
use tonic::Streaming;

use crate::error::CliError;
use crate::harness_session;
use crate::transport::connect_client;

pub enum StreamUpdate {
    Effects(Vec<UiEffect>),
    Completed,
    Failed(String),
}

pub async fn spawn_stream_task(
    prompt: String,
    mode: String,
    trace_id: String,
    harness_session_id: String,
) -> Result<mpsc::Receiver<StreamUpdate>, CliError> {
    let (tx, rx) = mpsc::channel(256);
    tokio::spawn(async move {
        if let Err(err) = run_stream(prompt, mode, trace_id, harness_session_id, tx.clone()).await {
            let _ = tx.send(StreamUpdate::Failed(err.to_string())).await;
        }
    });
    Ok(rx)
}

async fn run_stream(
    prompt: String,
    mode: String,
    trace_id: String,
    harness_session_id: String,
    tx: mpsc::Sender<StreamUpdate>,
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
    harness_session::insert_metadata(request.metadata_mut(), &harness_session_id)
        .map_err(CliError::Status)?;

    let response = client.stream_inference(request).await?;
    let mut stream: Streaming<rex_proto::rex::v1::StreamInferenceResponse> = response.into_inner();
    let mut consumer = StreamConsumer::new();

    while let Some(chunk) = stream.message().await.map_err(map_stream_err)? {
        if chunk.done {
            let _ = tx.send(StreamUpdate::Completed).await;
            return Ok(());
        }
        let effects = consumer.feed_grpc_chunk(&chunk);
        if !effects.is_empty() {
            let _ = tx.send(StreamUpdate::Effects(effects)).await;
        }
    }
    let _ = tx
        .send(StreamUpdate::Failed("stream ended without done".to_string()))
        .await;
    Ok(())
}

fn map_stream_err(status: tonic::Status) -> CliError {
    CliError::Status(status)
}

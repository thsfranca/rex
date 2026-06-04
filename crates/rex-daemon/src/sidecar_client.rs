//! gRPC client for `rex.sidecar.v1` over UDS.

use std::pin::Pin;
use std::time::Duration;

use async_stream::stream;
use hyper_util::rt::TokioIo;
use rex_proto::rex::sidecar::v1::sidecar_service_client::SidecarServiceClient;
use rex_proto::rex::sidecar::v1::{HealthRequest, RunTurnChunk, RunTurnRequest};
use rex_proto::rex::v1::StreamInferenceResponse;
use tokio::net::UnixStream;
use tokio_stream::Stream;
use tonic::transport::Endpoint;
use tonic::Status;
use tower::service_fn;

pub use crate::turn_correlation::TurnCorrelation;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

pub async fn connect_sidecar(
    socket_path: &str,
) -> Result<SidecarServiceClient<tonic::transport::Channel>, tonic::transport::Error> {
    let endpoint = Endpoint::try_from("http://[::]:50052")?.connect_timeout(CONNECT_TIMEOUT);
    let path = socket_path.to_string();
    let channel = endpoint
        .connect_with_connector(service_fn(move |_: tonic::transport::Uri| {
            let path = path.clone();
            async move { UnixStream::connect(path).await.map(TokioIo::new) }
        }))
        .await?;
    Ok(SidecarServiceClient::new(channel))
}

pub async fn health_check(
    client: &mut SidecarServiceClient<tonic::transport::Channel>,
) -> Result<bool, tonic::Status> {
    let response = client.health(HealthRequest {}).await?.into_inner();
    Ok(response.healthy)
}

fn run_turn_request(
    prompt: &str,
    mode: &str,
    model: &str,
    correlation: &TurnCorrelation,
) -> RunTurnRequest {
    RunTurnRequest {
        prompt: prompt.to_string(),
        mode: mode.to_string(),
        model: model.to_string(),
        turn_id: correlation.turn_id.clone(),
        context_revision: correlation.context_revision.clone(),
    }
}

pub fn map_run_turn_chunk(chunk: RunTurnChunk) -> StreamInferenceResponse {
    StreamInferenceResponse {
        text: chunk.text,
        index: chunk.index,
        done: chunk.done,
        event: chunk.event,
        tool_name: chunk.tool_name,
        phase: chunk.phase,
        summary: chunk.summary,
        detail: chunk.detail,
    }
}

pub async fn run_turn_collect(
    client: &mut SidecarServiceClient<tonic::transport::Channel>,
    prompt: &str,
    mode: &str,
    model: &str,
    correlation: &TurnCorrelation,
) -> Result<Vec<RunTurnChunk>, tonic::Status> {
    let request = run_turn_request(prompt, mode, model, correlation);
    let mut grpc_stream = client.run_turn(request).await?.into_inner();
    let mut chunks = Vec::new();
    while let Some(chunk) = grpc_stream.message().await? {
        chunks.push(chunk);
    }
    Ok(chunks)
}

pub type RunTurnInferenceStream =
    Pin<Box<dyn Stream<Item = Result<StreamInferenceResponse, Status>> + Send>>;

pub async fn run_turn_stream(
    client: &mut SidecarServiceClient<tonic::transport::Channel>,
    prompt: &str,
    mode: &str,
    model: &str,
    correlation: &TurnCorrelation,
) -> Result<RunTurnInferenceStream, tonic::Status> {
    let request = run_turn_request(prompt, mode, model, correlation);
    let mut grpc_stream = client.run_turn(request).await?.into_inner();
    Ok(Box::pin(stream! {
        while let Some(chunk) = grpc_stream.message().await? {
            yield Ok(map_run_turn_chunk(chunk));
        }
    }))
}

#[allow(clippy::result_large_err)]
pub fn map_sidecar_to_inference_chunks(
    chunks: Vec<RunTurnChunk>,
) -> Vec<Result<StreamInferenceResponse, Status>> {
    chunks
        .into_iter()
        .map(|c| Ok(map_run_turn_chunk(c)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::turn_correlation::build_turn_correlation;

    #[test]
    fn run_turn_request_populates_correlation_fields() {
        let correlation =
            build_turn_correlation(3, "ctx-body", "ran", "extractive_query", 1, false);
        let request = run_turn_request("hello", "ask", "model-a", &correlation);
        assert_eq!(request.turn_id, "turn-3");
        assert!(request.context_revision.starts_with("ctx-"));
    }

    #[test]
    fn map_run_turn_chunk_passes_structured_event_fields() {
        let chunk = RunTurnChunk {
            text: String::new(),
            index: 2,
            done: false,
            event: "tool".to_string(),
            tool_name: "fs.read".to_string(),
            phase: "running".to_string(),
            summary: String::new(),
            detail: "src/lib.rs".to_string(),
        };
        let mapped = map_run_turn_chunk(chunk);
        assert_eq!(mapped.event, "tool");
        assert_eq!(mapped.tool_name, "fs.read");
        assert_eq!(mapped.phase, "running");
        assert_eq!(mapped.detail, "src/lib.rs");
    }
}

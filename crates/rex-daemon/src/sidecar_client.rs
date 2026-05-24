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

fn run_turn_request(prompt: &str, mode: &str, model: &str) -> RunTurnRequest {
    RunTurnRequest {
        prompt: prompt.to_string(),
        mode: mode.to_string(),
        model: model.to_string(),
        turn_id: String::new(),
        context_revision: String::new(),
    }
}

pub fn map_run_turn_chunk(chunk: RunTurnChunk) -> StreamInferenceResponse {
    StreamInferenceResponse {
        text: chunk.text,
        index: chunk.index,
        done: chunk.done,
    }
}

pub async fn run_turn_collect(
    client: &mut SidecarServiceClient<tonic::transport::Channel>,
    prompt: &str,
    mode: &str,
    model: &str,
) -> Result<Vec<RunTurnChunk>, tonic::Status> {
    let request = run_turn_request(prompt, mode, model);
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
) -> Result<RunTurnInferenceStream, tonic::Status> {
    let request = run_turn_request(prompt, mode, model);
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

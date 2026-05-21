//! gRPC client for `rex.sidecar.v1` over UDS.

use std::time::Duration;

use hyper_util::rt::TokioIo;
use rex_proto::rex::sidecar::v1::sidecar_service_client::SidecarServiceClient;
use rex_proto::rex::sidecar::v1::{HealthRequest, RunTurnChunk, RunTurnRequest};
use rex_proto::rex::v1::StreamInferenceResponse;
use tokio::net::UnixStream;
use tonic::transport::Endpoint;
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

pub async fn run_turn_collect(
    client: &mut SidecarServiceClient<tonic::transport::Channel>,
    prompt: &str,
    mode: &str,
) -> Result<Vec<RunTurnChunk>, tonic::Status> {
    let request = RunTurnRequest {
        prompt: prompt.to_string(),
        mode: mode.to_string(),
    };
    let mut stream = client.run_turn(request).await?.into_inner();
    let mut chunks = Vec::new();
    while let Some(chunk) = stream.message().await? {
        chunks.push(chunk);
    }
    Ok(chunks)
}

#[allow(clippy::result_large_err)]
pub fn map_sidecar_to_inference_chunks(
    chunks: Vec<RunTurnChunk>,
) -> Vec<Result<StreamInferenceResponse, tonic::Status>> {
    chunks
        .into_iter()
        .map(|c| {
            Ok(StreamInferenceResponse {
                text: c.text,
                index: c.index,
                done: c.done,
            })
        })
        .collect()
}

//! gRPC client for `rex.capability.v1` over UDS.

use std::time::Duration;

use hyper_util::rt::TokioIo;
use rex_proto::rex::capability::v1::capability_service_client::CapabilityServiceClient;
use rex_proto::rex::capability::v1::HealthRequest;
use tokio::net::UnixStream;
use tonic::transport::Endpoint;
use tower::service_fn;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

pub async fn connect_capability(
    socket_path: &str,
) -> Result<CapabilityServiceClient<tonic::transport::Channel>, tonic::transport::Error> {
    let endpoint = Endpoint::try_from("http://[::]:50053")?.connect_timeout(CONNECT_TIMEOUT);
    let path = socket_path.to_string();
    let channel = endpoint
        .connect_with_connector(service_fn(move |_: tonic::transport::Uri| {
            let path = path.clone();
            async move { UnixStream::connect(path).await.map(TokioIo::new) }
        }))
        .await?;
    Ok(CapabilityServiceClient::new(channel))
}

pub async fn capability_health_check(
    client: &mut CapabilityServiceClient<tonic::transport::Channel>,
) -> Result<bool, tonic::Status> {
    let response = client.health(HealthRequest {}).await?.into_inner();
    Ok(response.healthy)
}

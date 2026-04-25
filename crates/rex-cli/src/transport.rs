use hyper_util::rt::TokioIo;
use rex_proto::rex::v1::rex_service_client::RexServiceClient;
use std::time::Duration;
use tokio::net::UnixStream;
use tonic::transport::Endpoint;
use tower::service_fn;

use crate::domain::{CONNECT_TIMEOUT_SECONDS, REQUEST_TIMEOUT_SECONDS, SOCKET_PATH};
use crate::error::CliError;

pub async fn connect_client() -> Result<RexServiceClient<tonic::transport::Channel>, CliError> {
    let endpoint = Endpoint::try_from("http://[::]:50051")?
        .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECONDS))
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS));
    let channel = endpoint
        .connect_with_connector(service_fn(|_: tonic::transport::Uri| async {
            UnixStream::connect(SOCKET_PATH).await.map(TokioIo::new)
        }))
        .await
        .map_err(|source| {
            let message = source.to_string();
            if message.contains("No such file")
                || message.contains("Connection refused")
                || message.contains("connection refused")
            {
                CliError::DaemonUnavailable {
                    socket_path: SOCKET_PATH.to_string(),
                }
            } else {
                CliError::DaemonConnect {
                    socket_path: SOCKET_PATH.to_string(),
                    source,
                }
            }
        })?;

    Ok(RexServiceClient::new(channel))
}

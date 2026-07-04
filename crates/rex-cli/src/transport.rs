use hyper_util::rt::TokioIo;
use rex_proto::rex::v1::rex_service_client::RexServiceClient;
use std::time::Duration;
use tokio::net::UnixStream;
use tonic::transport::Endpoint;
use tower::service_fn;

use crate::domain::{CONNECT_TIMEOUT_SECONDS, REQUEST_TIMEOUT_SECONDS, SOCKET_PATH};
use crate::error::CliError;

pub async fn connect_client(
    _trace_id: Option<&str>,
) -> Result<RexServiceClient<tonic::transport::Channel>, CliError> {
    let socket_path = daemon_socket_path();
    let connect_path = socket_path.clone();
    let endpoint = Endpoint::try_from("http://[::]:50051")?
        .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECONDS))
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS));
    let channel = endpoint
        .connect_with_connector(service_fn(move |_: tonic::transport::Uri| {
            let socket_path = connect_path.clone();
            async move { UnixStream::connect(socket_path).await.map(TokioIo::new) }
        }))
        .await
        .map_err(|source| {
            if is_daemon_unavailable_error(&source.to_string()) {
                CliError::DaemonUnavailable {
                    socket_path: socket_path.clone(),
                    suffix: "; enable `daemon.auto_start` or remove --no-daemon-autostart, then run `rex`"
                        .to_string(),
                }
            } else {
                CliError::DaemonConnect {
                    socket_path: socket_path.clone(),
                    source,
                }
            }
        })?;

    Ok(RexServiceClient::new(channel))
}

fn daemon_socket_path() -> String {
    rex_config::load_merged()
        .map(|loaded| loaded.daemon_socket().to_string())
        .unwrap_or_else(|_| SOCKET_PATH.to_string())
}

fn is_daemon_unavailable_error(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    message.contains("no such file")
        || message.contains("os error 2")
        || message.contains("connection refused")
        || message.contains("not connected")
}

#[cfg(test)]
mod tests {
    use super::is_daemon_unavailable_error;

    #[test]
    fn unavailable_detection_covers_missing_socket_paths() {
        assert!(is_daemon_unavailable_error(
            "No such file or directory (os error 2)"
        ));
    }

    #[test]
    fn unavailable_detection_covers_connection_refused() {
        assert!(is_daemon_unavailable_error(
            "transport error: Connection refused"
        ));
    }

    #[test]
    fn unavailable_detection_ignores_unrelated_transport_errors() {
        assert!(!is_daemon_unavailable_error("invalid URI format"));
    }
}

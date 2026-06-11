use std::io;
use std::path::Path;

use rex_proto::rex::capability::v1::capability_service_server::{
    CapabilityService, CapabilityServiceServer,
};
use rex_proto::rex::capability::v1::{
    GetCapabilitiesRequest, GetCapabilitiesResponse, HealthRequest, HealthResponse, InvokeRequest,
    InvokeResponse,
};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

pub const DEFAULT_SOCKET_PATH: &str = "/tmp/rex-capability-mock.sock";
pub const CAPABILITY_MOCK_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Default)]
pub struct MockCapability;

#[tonic::async_trait]
impl CapabilityService for MockCapability {
    async fn health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        Ok(Response::new(HealthResponse {
            healthy: true,
            version: CAPABILITY_MOCK_VERSION.to_string(),
        }))
    }

    async fn get_capabilities(
        &self,
        _request: Request<GetCapabilitiesRequest>,
    ) -> Result<Response<GetCapabilitiesResponse>, Status> {
        Ok(Response::new(GetCapabilitiesResponse {
            capability_ids: vec!["web.search".to_string()],
        }))
    }

    async fn invoke(
        &self,
        _request: Request<InvokeRequest>,
    ) -> Result<Response<InvokeResponse>, Status> {
        Err(Status::unimplemented("Invoke deferred to R056-3"))
    }
}

pub fn remove_stale_socket(path: &str) -> io::Result<()> {
    let p = Path::new(path);
    if p.exists() {
        std::fs::remove_file(p)?;
    }
    Ok(())
}

pub async fn serve_on_socket(
    socket_path: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    remove_stale_socket(socket_path)?;
    let listener = UnixListener::bind(socket_path)?;
    let incoming = UnixListenerStream::new(listener);
    eprintln!(
        "rex-capability-mock event=listen socket={} version={}",
        socket_path, CAPABILITY_MOCK_VERSION
    );
    Server::builder()
        .add_service(CapabilityServiceServer::new(MockCapability))
        .serve_with_incoming(incoming)
        .await?;
    let _ = remove_stale_socket(socket_path);
    Ok(())
}

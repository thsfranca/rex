use std::env;

use rex_sidecar_stub::{serve_on_socket, DEFAULT_SOCKET_PATH};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let socket_path = env::var("REX_SIDECAR_SOCKET")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_SOCKET_PATH.to_string());
    tokio::select! {
        result = serve_on_socket(&socket_path) => {
            result.map_err(|e| e.to_string())?;
        }
        _ = tokio::signal::ctrl_c() => {
            eprintln!("rex-sidecar-stub event=shutdown socket={}", socket_path);
        }
    }
    Ok(())
}

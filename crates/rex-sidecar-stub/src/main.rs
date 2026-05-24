use rex_sidecar_stub::serve_on_socket;

fn resolve_sidecar_socket() -> String {
    if let Ok(raw) = std::env::var("REX_SIDECAR_SOCKET") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    rex_config::load_merged()
        .ok()
        .and_then(|loaded| loaded.active_sidecar().map(|entry| entry.socket.clone()))
        .unwrap_or_else(|| rex_sidecar_stub::DEFAULT_SOCKET_PATH.to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let socket_path = resolve_sidecar_socket();
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

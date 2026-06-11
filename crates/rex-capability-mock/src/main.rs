use rex_capability_mock::serve_on_socket;

fn resolve_socket() -> String {
    if let Ok(raw) = std::env::var("REX_SIDECAR_SOCKET") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    rex_capability_mock::DEFAULT_SOCKET_PATH.to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let socket_path = resolve_socket();
    tokio::select! {
        result = serve_on_socket(&socket_path) => {
            result.map_err(|e| e.to_string())?;
        }
        _ = tokio::signal::ctrl_c() => {
            eprintln!("rex-capability-mock event=shutdown socket={}", socket_path);
        }
    }
    Ok(())
}

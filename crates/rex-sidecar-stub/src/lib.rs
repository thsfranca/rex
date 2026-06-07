use std::io;
use std::path::Path;
use std::time::Duration;

use hyper_util::rt::TokioIo;
use rex_proto::rex::v1::rex_service_client::RexServiceClient;
use rex_proto::rex::v1::{
    BrokerExecShellRequest, BrokerInferenceRequest, BrokerListDirRequest, BrokerReadFileRequest,
    BrokerWriteFileRequest,
};
use tokio::net::UnixStream;
use tonic::transport::Endpoint;
use tower::service_fn;

use rex_proto::rex::sidecar::v1::sidecar_service_server::{SidecarService, SidecarServiceServer};
use rex_proto::rex::sidecar::v1::{
    GetCapabilitiesRequest, GetCapabilitiesResponse, HealthRequest, HealthResponse, RunTurnChunk,
    RunTurnRequest,
};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

pub const DEFAULT_SOCKET_PATH: &str = "/tmp/rex-sidecar.sock";
pub const SIDECAR_VERSION: &str = env!("CARGO_PKG_VERSION");
const CHUNK_DELAY_MS: u64 = 5;

#[derive(Default)]
pub struct StubSidecar;

#[tonic::async_trait]
impl SidecarService for StubSidecar {
    async fn health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        Ok(Response::new(HealthResponse {
            healthy: true,
            version: SIDECAR_VERSION.to_string(),
        }))
    }

    async fn get_capabilities(
        &self,
        _request: Request<GetCapabilitiesRequest>,
    ) -> Result<Response<GetCapabilitiesResponse>, Status> {
        Ok(Response::new(GetCapabilitiesResponse {
            capabilities: vec!["run_turn".to_string()],
        }))
    }

    type RunTurnStream =
        std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<RunTurnChunk, Status>> + Send>>;

    async fn run_turn(
        &self,
        request: Request<RunTurnRequest>,
    ) -> Result<Response<Self::RunTurnStream>, Status> {
        let inner = request.into_inner();
        let mode = if inner.mode.trim().is_empty() {
            "ask"
        } else {
            inner.mode.trim()
        };
        let mut text = match broker_inference(&inner.prompt, mode, &inner.model).await {
            Ok(content) => content,
            Err(err) => {
                let stream = async_stream::stream! {
                    yield Ok(RunTurnChunk {
                        text: format!("[broker.inference error: {err}]"),
                        index: 0,
                        done: false,
                        ..Default::default()
                    });
                    yield Ok(RunTurnChunk {
                        text: String::new(),
                        index: 1,
                        done: true,
                        ..Default::default()
                    });
                };
                return Ok(Response::new(Box::pin(stream)));
            }
        };
        if let Some(path) = parse_read_directive(&inner.prompt) {
            match broker_read_file(&path, mode).await {
                Ok(content) => {
                    text.push_str(&format!("\n\n[fs.read:{path}]\n{content}"));
                }
                Err(err) => {
                    text.push_str(&format!("\n\n[fs.read error:{err}]"));
                }
            }
        }
        if let Some(path) = parse_list_directive(&inner.prompt) {
            match broker_list_dir(&path, mode).await {
                Ok(entries) => {
                    let listing = entries
                        .iter()
                        .map(|entry| {
                            if entry.is_dir {
                                format!("{}/", entry.name)
                            } else {
                                entry.name.clone()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    text.push_str(&format!("\n\n[fs.list:{path}]\n{listing}"));
                }
                Err(err) => {
                    text.push_str(&format!("\n\n[fs.list error:{err}]"));
                }
            }
        }
        if let Some(command) = parse_exec_directive(&inner.prompt) {
            match broker_exec_shell(&command, mode).await {
                Ok(out) => {
                    text.push_str(&format!(
                        "\n\n[exec.shell:{command}]\nstdout={}\nstderr={}",
                        out.stdout, out.stderr
                    ));
                }
                Err(err) => {
                    text.push_str(&format!("\n\n[exec.shell error:{err}]"));
                }
            }
        }
        if let Some((path, content)) = parse_write_directive(&inner.prompt) {
            match broker_write_file(&path, &content, mode).await {
                Ok(()) => {
                    text.push_str(&format!("\n\n[fs.write:{path}] ok"));
                }
                Err(err) => {
                    text.push_str(&format!("\n\n[fs.write error:{err}]"));
                }
            }
        }
        let chunks = chunk_text(&text, 8);
        let terminal_index = chunks.len() as u64;
        let stream = async_stream::stream! {
            for (index, piece) in chunks.into_iter().enumerate() {
                tokio::time::sleep(Duration::from_millis(CHUNK_DELAY_MS)).await;
                yield Ok(RunTurnChunk {
                    text: piece,
                    index: index as u64,
                    done: false,
                    ..Default::default()
                });
            }
            yield Ok(RunTurnChunk {
                text: String::new(),
                index: terminal_index,
                done: true,
                ..Default::default()
            });
        };
        Ok(Response::new(Box::pin(stream)))
    }
}

fn daemon_socket_path() -> String {
    if let Ok(raw) = std::env::var("REX_DAEMON_SOCKET") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    rex_config::load_merged()
        .map(|loaded| loaded.daemon_socket().to_string())
        .unwrap_or_else(|_| "/tmp/rex.sock".to_string())
}

async fn connect_daemon(
    socket: &str,
) -> Result<RexServiceClient<tonic::transport::Channel>, String> {
    let endpoint = Endpoint::try_from("http://[::]:50051")
        .map_err(|e| e.to_string())?
        .connect_timeout(Duration::from_secs(2));
    let path_socket = socket.to_string();
    let channel = endpoint
        .connect_with_connector(service_fn(move |_: tonic::transport::Uri| {
            let path_socket = path_socket.clone();
            async move {
                UnixStream::connect(path_socket)
                    .await
                    .map(TokioIo::new)
                    .map_err(std::io::Error::other)
            }
        }))
        .await
        .map_err(|e| e.to_string())?;
    Ok(RexServiceClient::new(channel))
}

async fn broker_inference(prompt: &str, mode: &str, model: &str) -> Result<String, String> {
    let socket = daemon_socket_path();
    let mut client = connect_daemon(&socket).await?;
    let response = client
        .broker_inference(BrokerInferenceRequest {
            prompt: prompt.to_string(),
            mode: mode.to_string(),
            model: model.to_string(),
            messages: Vec::new(),
            tools: Vec::new(),
        })
        .await
        .map_err(|e| e.to_string())?
        .into_inner();
    if response.ok {
        Ok(response.text)
    } else {
        Err(response.error)
    }
}

fn parse_read_directive(prompt: &str) -> Option<String> {
    parse_path_directive(prompt, "__rex_read:")
}

fn parse_list_directive(prompt: &str) -> Option<String> {
    parse_path_directive(prompt, "__rex_list:")
}

fn parse_path_directive(prompt: &str, marker: &str) -> Option<String> {
    let start = prompt.find(marker)? + marker.len();
    let rest = &prompt[start..];
    let path = rest
        .split_whitespace()
        .next()
        .or_else(|| rest.split('\n').next())?
        .trim();
    Some(path.to_string())
}

fn parse_exec_directive(prompt: &str) -> Option<String> {
    let marker = "__rex_exec:";
    let start = prompt.find(marker)? + marker.len();
    let command = prompt[start..].lines().next()?.trim();
    if command.is_empty() {
        None
    } else {
        Some(command.to_string())
    }
}

async fn broker_exec_shell(command: &str, mode: &str) -> Result<ShellOut, String> {
    let socket = daemon_socket_path();
    let mut client = connect_daemon(&socket).await?;
    let response = client
        .broker_exec_shell(BrokerExecShellRequest {
            command: command.to_string(),
            mode: mode.to_string(),
        })
        .await
        .map_err(|e| e.to_string())?
        .into_inner();
    if response.ok {
        Ok(ShellOut {
            stdout: response.stdout,
            stderr: response.stderr,
        })
    } else {
        Err(response.error)
    }
}

struct ShellOut {
    stdout: String,
    stderr: String,
}

fn parse_write_directive(prompt: &str) -> Option<(String, String)> {
    let marker = "__rex_write:";
    let start = prompt.find(marker)? + marker.len();
    let rest = prompt[start..].trim_start();
    let (path, content) = rest.split_once('\n')?;
    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    Some((path.to_string(), content.to_string()))
}

async fn broker_write_file(path: &str, content: &str, mode: &str) -> Result<(), String> {
    let socket = daemon_socket_path();
    let mut client = connect_daemon(&socket).await?;
    let response = client
        .broker_write_file(BrokerWriteFileRequest {
            path: path.to_string(),
            content: content.to_string(),
            mode: mode.to_string(),
        })
        .await
        .map_err(|e| e.to_string())?
        .into_inner();
    if response.ok {
        Ok(())
    } else {
        Err(response.error)
    }
}

async fn broker_read_file(path: &str, mode: &str) -> Result<String, String> {
    let socket = daemon_socket_path();
    let mut client = connect_daemon(&socket).await?;
    let response = client
        .broker_read_file(BrokerReadFileRequest {
            path: path.to_string(),
            mode: mode.to_string(),
        })
        .await
        .map_err(|e| e.to_string())?
        .into_inner();
    if response.ok {
        Ok(response.content)
    } else {
        Err(response.error)
    }
}

struct BrokerListEntry {
    name: String,
    is_dir: bool,
}

async fn broker_list_dir(path: &str, mode: &str) -> Result<Vec<BrokerListEntry>, String> {
    let socket = daemon_socket_path();
    let mut client = connect_daemon(&socket).await?;
    let response = client
        .broker_list_dir(BrokerListDirRequest {
            path: path.to_string(),
            mode: mode.to_string(),
        })
        .await
        .map_err(|e| e.to_string())?
        .into_inner();
    if response.ok {
        Ok(response
            .entries
            .into_iter()
            .map(|entry| BrokerListEntry {
                name: entry.name,
                is_dir: entry.is_dir,
            })
            .collect())
    } else {
        Err(response.error)
    }
}

fn chunk_text(text: &str, max_chars: usize) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let size = max_chars.max(1);
    text.chars()
        .collect::<Vec<_>>()
        .chunks(size)
        .map(|c| c.iter().collect())
        .collect()
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
        "rex-sidecar-stub event=listen socket={} version={}",
        socket_path, SIDECAR_VERSION
    );
    Server::builder()
        .add_service(SidecarServiceServer::new(StubSidecar))
        .serve_with_incoming(incoming)
        .await?;
    let _ = remove_stale_socket(socket_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_text_splits_non_empty() {
        assert_eq!(chunk_text("abcdef", 2), vec!["ab", "cd", "ef"]);
    }

    #[test]
    fn parse_read_directive_extracts_path() {
        let prompt = "hello\n__rex_read: src/main.rs\nmore";
        assert_eq!(parse_read_directive(prompt).as_deref(), Some("src/main.rs"));
    }

    #[test]
    fn parse_write_directive_extracts_path_and_body() {
        let prompt = "__rex_write: out.txt\nline one\nline two";
        assert_eq!(
            parse_write_directive(prompt),
            Some(("out.txt".to_string(), "line one\nline two".to_string()))
        );
    }
}

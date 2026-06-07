use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use futures::StreamExt;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::{Bytes, Frame};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use rex_obs_store::{
    instrument_catalog_with_extensions, project_metrics, rollup_metrics_by_label, tail_telemetry,
    MetricsQueryRequest, MetricsRollupRequest, ObsQuery, StoreEngine, StorePort, StreamQueryFilter,
};
use tokio::net::TcpListener;
use tokio_stream::wrappers::ReceiverStream;

use crate::error::ReadApiError;

type ApiBody = BoxBody<Bytes, Infallible>;
type ApiResponse = Response<ApiBody>;

pub struct ReadApiState {
    pub service_name: String,
    store: Arc<Mutex<StoreEngine>>,
    tail: Arc<rex_obs_store::TelemetryTail>,
}

impl ReadApiState {
    pub fn new(service_name: String, store: StoreEngine) -> Self {
        let tail = Arc::clone(store.tail());
        Self {
            service_name,
            store: Arc::new(Mutex::new(store)),
            tail,
        }
    }
}

pub async fn serve(listen: &str, state: ReadApiState) -> Result<(), ReadApiError> {
    let addr: SocketAddr = listen
        .parse()
        .map_err(|err| ReadApiError::BindFailed(format!("invalid listen address: {err}")))?;
    if !addr.ip().is_loopback() {
        return Err(ReadApiError::BindFailed(format!(
            "obs.read_api.bind_failed: non-loopback address {}",
            addr.ip()
        )));
    }

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|err| ReadApiError::BindFailed(err.to_string()))?;
    let shared = Arc::new(state);

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let shared = Arc::clone(&shared);
        tokio::spawn(async move {
            let service = service_fn(move |req| {
                let shared = Arc::clone(&shared);
                async move { handle_request(req, shared).await }
            });
            if http1::Builder::new()
                .serve_connection(io, service)
                .await
                .is_err()
            {}
        });
    }
}

async fn handle_request(
    req: Request<hyper::body::Incoming>,
    state: Arc<ReadApiState>,
) -> Result<ApiResponse, Infallible> {
    let path = req.uri().path();
    let response: ApiResponse = match (req.method(), path) {
        (&Method::GET, "/health") => json_response(
            StatusCode::OK,
            &serde_json::json!({ "status": "ok", "service": "rex-obs-read-api" }),
        ),
        (&Method::GET, "/v1/catalog") => handle_catalog(&state),
        (&Method::POST, "/v1/metrics/query") => match read_body(req).await {
            Ok(body) => handle_metrics_query(&state, &body),
            Err(err) => error_response(StatusCode::BAD_REQUEST, &err.to_string()),
        },
        (&Method::POST, "/v1/metrics/rollup") => match read_body(req).await {
            Ok(body) => handle_metrics_rollup(&state, &body),
            Err(err) => error_response(StatusCode::BAD_REQUEST, &err.to_string()),
        },
        (&Method::GET, "/v1/metrics/stream") => handle_metrics_stream(&state, req.uri().query()),
        _ => error_response(StatusCode::NOT_FOUND, "not found"),
    };
    Ok(response)
}

fn handle_catalog(state: &ReadApiState) -> ApiResponse {
    let sidecar = match state.store.lock() {
        Ok(store) => store.list_sidecar_metrics().unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    json_response(
        StatusCode::OK,
        &serde_json::json!({ "instruments": instrument_catalog_with_extensions(&sidecar) }),
    )
}

fn handle_metrics_query(state: &ReadApiState, body: &str) -> ApiResponse {
    let request: MetricsQueryRequest = match serde_json::from_str(body) {
        Ok(req) => req,
        Err(err) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("obs.read_api.query_invalid: {err}"),
            );
        }
    };
    let filter = StreamQueryFilter {
        start_ms: request.start_ms,
        end_ms: request.end_ms,
        terminal: request.labels.get("terminal").cloned(),
        route: request.labels.get("route").cloned(),
        mode: request.labels.get("mode").cloned(),
        cache_decision: request.labels.get("decision").cloned(),
    };
    let streams = match state.store.lock() {
        Ok(store) => match store.query_streams(&filter) {
            Ok(rows) => rows,
            Err(err) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("store query failed: {err}"),
                );
            }
        },
        Err(_) => {
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "store lock poisoned");
        }
    };
    let response = project_metrics(&state.service_name, &streams, &request);
    json_response(StatusCode::OK, &response)
}

fn handle_metrics_rollup(state: &ReadApiState, body: &str) -> ApiResponse {
    let request: MetricsRollupRequest = match serde_json::from_str(body) {
        Ok(req) => req,
        Err(err) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                &format!("obs.read_api.query_invalid: {err}"),
            );
        }
    };
    if request.group_by.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "obs.read_api.query_invalid: group_by required",
        );
    }
    let filter = StreamQueryFilter {
        start_ms: request.start_ms,
        end_ms: request.end_ms,
        terminal: None,
        route: None,
        mode: None,
        cache_decision: None,
    };
    let streams = match state.store.lock() {
        Ok(store) => match store.query_streams(&filter) {
            Ok(rows) => rows,
            Err(err) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("store query failed: {err}"),
                );
            }
        },
        Err(_) => {
            return error_response(StatusCode::INTERNAL_SERVER_ERROR, "store lock poisoned");
        }
    };
    let response = rollup_metrics_by_label(&streams, &request);
    json_response(StatusCode::OK, &response)
}

fn handle_metrics_stream(state: &Arc<ReadApiState>, query: Option<&str>) -> ApiResponse {
    let params = parse_query(query);
    let cursor_raw = match params.get("cursor_commit_ms") {
        Some(v) => v.as_str(),
        None => {
            return stream_error_response(
                StatusCode::BAD_REQUEST,
                "obs.read_api.stream_invalid: cursor_commit_ms required",
            );
        }
    };
    let cursor_commit_ms: i64 = match cursor_raw.parse() {
        Ok(v) => v,
        Err(_) => {
            return stream_error_response(
                StatusCode::BAD_REQUEST,
                "obs.read_api.stream_invalid: cursor_commit_ms must be integer",
            );
        }
    };
    let instruments = params.get("instruments").map(|raw| {
        raw.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>()
    });

    let (tx, rx) = tokio::sync::mpsc::channel(32);
    let state = Arc::clone(state);
    tokio::spawn(async move {
        let mut events = std::pin::pin!(tail_telemetry(
            &state.tail,
            Arc::clone(&state.store),
            &state.service_name,
            cursor_commit_ms,
            instruments,
        ));
        while let Some(event) = events.next().await {
            let payload = match serde_json::to_string(&event) {
                Ok(json) => format!("data: {json}\n\n"),
                Err(_) => continue,
            };
            if tx
                .send(Ok(Frame::data(Bytes::from(payload))))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/event-stream")
        .header("cache-control", "no-cache")
        .body(
            StreamBody::new(ReceiverStream::new(rx))
                .map_err(|never: Infallible| match never {})
                .boxed(),
        )
        .unwrap_or_else(|_| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "obs.read_api.stream_invalid: response build failed",
            )
        })
}

fn parse_query(query: Option<&str>) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    if let Some(q) = query {
        for pair in q.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                out.insert(k.to_string(), v.to_string());
            }
        }
    }
    out
}

async fn read_body(req: Request<hyper::body::Incoming>) -> Result<String, ReadApiError> {
    let collected = req
        .into_body()
        .collect()
        .await
        .map_err(|err| ReadApiError::Http(err.to_string()))?;
    let bytes = collected.to_bytes();
    String::from_utf8(bytes.to_vec()).map_err(|err| ReadApiError::QueryInvalid(err.to_string()))
}

fn json_response<T: serde::Serialize>(status: StatusCode, value: &T) -> ApiResponse {
    let body = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(
            Full::new(Bytes::from(body))
                .map_err(|never| match never {})
                .boxed(),
        )
        .unwrap_or_else(|_| {
            Response::new(
                Full::new(Bytes::from("{}"))
                    .map_err(|never| match never {})
                    .boxed(),
            )
        })
}

fn error_response(status: StatusCode, message: &str) -> ApiResponse {
    json_response(
        status,
        &serde_json::json!({ "error": message, "code": "obs.read_api.query_invalid" }),
    )
}

fn stream_error_response(status: StatusCode, message: &str) -> ApiResponse {
    json_response(
        status,
        &serde_json::json!({ "error": message, "code": "obs.read_api.stream_invalid" }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rex_obs_store::{StorePort, StreamEconomicsRecord};

    fn sample(snapshot_id: &str) -> StreamEconomicsRecord {
        StreamEconomicsRecord {
            snapshot_id: snapshot_id.to_string(),
            request_id: 1,
            trace_id: "trace-1".to_string(),
            turn_id: "".to_string(),
            terminal: "done".to_string(),
            route: "sidecar+mock".to_string(),
            cache_decision: "miss_stored".to_string(),
            decision_id: "dec-1".to_string(),
            inference_runtime: "mock".to_string(),
            mode: "ask".to_string(),
            model: "gpt-4o-mini".to_string(),
            elapsed_ms: 42,
            chunks_sent: 1,
            prompt_tokens: 10,
            context_tokens: 5,
            context_candidates: 1,
            context_selected: 1,
            context_truncated: false,
            retrieval: "skipped".to_string(),
            compression_strategy: "none".to_string(),
            cached_tokens: None,
            prefix_hash: None,
            parse_retries: None,
        }
    }

    #[test]
    fn metrics_query_projects_counter() {
        let dir = tempfile::tempdir().unwrap();
        let store = rex_obs_store::open_store("sqlite", dir.path().join("store.sqlite")).unwrap();
        store
            .upsert_config_snapshot("snap", r#"{"inference":{"runtime":"mock"}}"#)
            .unwrap();
        store.append_stream(&sample("snap")).unwrap();
        let state = ReadApiState::new("rex-daemon".to_string(), store);
        let body = r#"{"instruments":["rex.stream.requests"]}"#;
        let resp = handle_metrics_query(&state, body);
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[test]
    fn stream_requires_cursor_query_param() {
        let dir = tempfile::tempdir().unwrap();
        let store = rex_obs_store::open_store("sqlite", dir.path().join("store.sqlite")).unwrap();
        let state = Arc::new(ReadApiState::new("rex-daemon".to_string(), store));
        let resp = handle_metrics_stream(&state, None);
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}

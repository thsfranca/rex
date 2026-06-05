use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use rex_obs_store::{
    instrument_catalog, project_metrics, MetricsQueryRequest, ObsQuery, ObsStore, StreamQueryFilter,
};
use tokio::net::TcpListener;

use crate::error::ReadApiError;

pub struct ReadApiState {
    pub service_name: String,
    store: Mutex<ObsStore>,
}

impl ReadApiState {
    pub fn new(service_name: String, store: ObsStore) -> Self {
        Self {
            service_name,
            store: Mutex::new(store),
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
) -> Result<Response<Full<Bytes>>, Infallible> {
    let path = req.uri().path();
    let response = match (req.method(), path) {
        (&Method::GET, "/health") => json_response(
            StatusCode::OK,
            &serde_json::json!({ "status": "ok", "service": "rex-obs-read-api" }),
        ),
        (&Method::GET, "/v1/catalog") => json_response(
            StatusCode::OK,
            &serde_json::json!({ "instruments": instrument_catalog() }),
        ),
        (&Method::POST, "/v1/metrics/query") => match read_body(req).await {
            Ok(body) => handle_metrics_query(&state, &body),
            Err(err) => error_response(StatusCode::BAD_REQUEST, &err.to_string()),
        },
        _ => error_response(StatusCode::NOT_FOUND, "not found"),
    };
    Ok(response)
}

fn handle_metrics_query(state: &ReadApiState, body: &str) -> Response<Full<Bytes>> {
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

async fn read_body(req: Request<hyper::body::Incoming>) -> Result<String, ReadApiError> {
    let collected = req
        .into_body()
        .collect()
        .await
        .map_err(|err| ReadApiError::Http(err.to_string()))?;
    let bytes = collected.to_bytes();
    String::from_utf8(bytes.to_vec()).map_err(|err| ReadApiError::QueryInvalid(err.to_string()))
}

fn json_response<T: serde::Serialize>(status: StatusCode, value: &T) -> Response<Full<Bytes>> {
    let body = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap_or_else(|_| Response::new(Full::new(Bytes::from("{}"))))
}

fn error_response(status: StatusCode, message: &str) -> Response<Full<Bytes>> {
    json_response(
        status,
        &serde_json::json!({ "error": message, "code": "obs.read_api.query_invalid" }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rex_obs_store::StreamEconomicsRecord;

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
        let store = ObsStore::open(dir.path().join("store.sqlite")).unwrap();
        store
            .upsert_config_snapshot("snap", r#"{"inference":{"runtime":"mock"}}"#)
            .unwrap();
        store.append_stream(&sample("snap")).unwrap();
        let state = ReadApiState::new("rex-daemon".to_string(), store);
        let body = r#"{"instruments":["rex.stream.requests"]}"#;
        let resp = handle_metrics_query(&state, body);
        assert_eq!(resp.status(), StatusCode::OK);
    }
}

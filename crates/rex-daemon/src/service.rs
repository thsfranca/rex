use std::time::Instant;
use std::{
    env,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use async_stream::stream;
use rex_proto::rex::v1::rex_service_server::RexService;
use rex_proto::rex::v1::{
    GetSystemStatusRequest, GetSystemStatusResponse, StreamInferenceRequest,
    StreamInferenceResponse,
};
use tokio::time::{sleep, Duration};
use tokio_stream::Stream;
use tonic::{Request, Response, Status};

use crate::adapters::InferenceRuntime;
#[cfg(test)]
use crate::adapters::MockInferenceRuntime;
use crate::adapters::RuntimeKind;
use crate::domain::{StreamLifecycle, ACTIVE_MODEL_ID, DAEMON_VERSION};
use crate::l1_cache::{l1_cachable_responses, normalize_mode};
use crate::plugins::{
    BehaviorDecision, BehaviorSnapshot, CacheStatus, ContextPipeline, ContextRequest,
};
use crate::policy::{CacheDecision, PolicyEngine, PolicyRequest};

pub struct RexDaemonService {
    started_at: Instant,
    pipeline: Mutex<ContextPipeline>,
    runtime: Arc<dyn InferenceRuntime>,
    request_sequence: AtomicU64,
    policy: PolicyEngine,
}

const STREAM_CHUNK_DELAY_MS: u64 = 35;

impl RexDaemonService {
    pub fn with_runtime(started_at: Instant, runtime: Arc<dyn InferenceRuntime>) -> Self {
        Self::with_runtime_and_policy(started_at, runtime, PolicyEngine::with_default_l1())
    }

    /// Test-friendly constructor: inject any `PolicyEngine` (and therefore any
    /// `ResponseCache`) so tests can observe cache call ordering without spinning
    /// up the production L1 LRU.
    pub fn with_runtime_and_policy(
        started_at: Instant,
        runtime: Arc<dyn InferenceRuntime>,
        policy: PolicyEngine,
    ) -> Self {
        Self {
            started_at,
            pipeline: Mutex::new(ContextPipeline::default_sidecar_like()),
            runtime,
            request_sequence: AtomicU64::new(1),
            policy,
        }
    }

    #[cfg(test)]
    pub async fn build_inference_chunks(
        prompt: &str,
    ) -> Vec<Result<StreamInferenceResponse, Status>> {
        MockInferenceRuntime.build_chunks(prompt).await
    }
}

#[tonic::async_trait]
impl RexService for RexDaemonService {
    async fn get_system_status(
        &self,
        _request: Request<GetSystemStatusRequest>,
    ) -> Result<Response<GetSystemStatusResponse>, Status> {
        Ok(Response::new(GetSystemStatusResponse {
            daemon_version: DAEMON_VERSION.to_string(),
            uptime_seconds: self.started_at.elapsed().as_secs(),
            active_model_id: ACTIVE_MODEL_ID.to_string(),
        }))
    }

    type StreamInferenceStream =
        std::pin::Pin<Box<dyn Stream<Item = Result<StreamInferenceResponse, Status>> + Send>>;

    async fn stream_inference(
        &self,
        request: Request<StreamInferenceRequest>,
    ) -> Result<Response<Self::StreamInferenceStream>, Status> {
        let request_started = Instant::now();
        let request_id = self.request_sequence.fetch_add(1, Ordering::Relaxed);
        let trace_id = extract_trace_id(request.metadata(), request_id);
        let inference_runtime = RuntimeKind::from_env().log_label();
        let inner = request.into_inner();
        let prompt = inner.prompt;
        let model = inner.model;
        let mode = inner.mode;
        let directives = PromptDirectives::from_prompt(&prompt);
        let context_request = ContextRequest {
            prompt: prompt.clone(),
            diagnostics_hint: directives.diagnostics_hint.clone(),
            cache_bypass: directives.cache_bypass || cache_bypass_from_env(),
            behavior_snapshot: directives.behavior_snapshot,
        };
        let pipeline_result = self
            .pipeline
            .lock()
            .expect("context pipeline mutex should not be poisoned")
            .prepare(&context_request);
        let prompt_len = prompt.chars().count();
        println!(
            "stream.request_id={request_id} trace_id={trace_id} inference_runtime={inference_runtime} stream.lifecycle={} prompt_len={prompt_len}",
            StreamLifecycle::Starting.as_str(),
        );
        println!(
            "stream.request_id={request_id} trace_id={trace_id} inference_runtime={inference_runtime} stream.metrics prompt_tokens={} context_tokens={} candidates={} selected={} truncated={} cache={} behavior={}",
            pipeline_result.metrics.prompt_tokens,
            pipeline_result.metrics.selected_context_tokens,
            pipeline_result.metrics.context_candidates,
            pipeline_result.metrics.context_selected,
            pipeline_result.metrics.context_truncated,
            format_cache_status(pipeline_result.metrics.cache_status),
            format_behavior_decision(&pipeline_result.metrics.behavior_decision),
        );
        let cache_bypass = directives.cache_bypass || cache_bypass_from_env();
        let policy_request = PolicyRequest {
            runtime: RuntimeKind::from_env(),
            model: &model,
            mode: &mode,
            effective_prompt: &pipeline_result.effective_prompt,
            cache_bypass,
        };
        let decision = self.policy.decide(&policy_request);
        let mut l1_state: Option<&'static str> = None;
        let chunks: Vec<Result<StreamInferenceResponse, Status>> = match &decision {
            CacheDecision::Lookup(key) => {
                if let Some(cached) = self.policy.get(key) {
                    l1_state = Some("hit");
                    cached.into_iter().map(Ok).collect()
                } else {
                    l1_state = Some("miss");
                    let built = self
                        .runtime
                        .build_chunks(&pipeline_result.effective_prompt)
                        .await;
                    if let Some(to_store) = l1_cachable_responses(&built) {
                        self.policy.put(key.clone(), to_store);
                    }
                    built
                }
            }
            CacheDecision::Bypass | CacheDecision::Uncacheable { .. } => {
                self.runtime
                    .build_chunks(&pipeline_result.effective_prompt)
                    .await
            }
        };
        if let Some(state) = l1_state {
            println!(
                "stream.request_id={request_id} trace_id={trace_id} inference_runtime={inference_runtime} l1_cache={state} model={} mode={}",
                if model.trim().is_empty() {
                    ACTIVE_MODEL_ID
                } else {
                    model.trim()
                },
                normalize_mode(&mode)
            );
        }
        println!(
            "stream.request_id={request_id} trace_id={trace_id} inference_runtime={inference_runtime} stream.lifecycle={} chunk_count={}",
            StreamLifecycle::Streaming.as_str(),
            chunks.len()
        );
        let output = stream! {
            let mut chunk_count: u64 = 0;
            let mut done_seen = false;
            for chunk in chunks {
                match chunk {
                    Ok(chunk) => {
                        chunk_count += 1;
                        if chunk_count == 1 {
                            println!(
                                "stream.request_id={request_id} trace_id={trace_id} inference_runtime={inference_runtime} stream.event=first_chunk index={} done={}",
                                chunk.index,
                                chunk.done
                            );
                        }
                        if chunk.done {
                            done_seen = true;
                        }
                        let terminal = chunk.done;
                        yield Ok(chunk);
                        if !terminal {
                            sleep(Duration::from_millis(STREAM_CHUNK_DELAY_MS)).await;
                        }
                    }
                    Err(err) => {
                        println!(
                            "stream.request_id={request_id} trace_id={trace_id} inference_runtime={inference_runtime} stream.lifecycle={} stream.event=error stream.terminal=grpc_error grpc_code={} message={} elapsed_ms={}",
                            StreamLifecycle::Failed.as_str(),
                            err.code() as i32,
                            err.message(),
                            request_started.elapsed().as_millis()
                        );
                        yield Err(err);
                        return;
                    }
                }
            }
            if done_seen {
                println!(
                    "stream.request_id={request_id} trace_id={trace_id} inference_runtime={inference_runtime} stream.lifecycle={} stream.terminal=done chunks_sent={chunk_count} elapsed_ms={}",
                    StreamLifecycle::Completed.as_str(),
                    request_started.elapsed().as_millis()
                );
            } else {
                println!(
                    "stream.request_id={request_id} trace_id={trace_id} inference_runtime={inference_runtime} stream.lifecycle={} stream.event=incomplete stream.terminal=missing_done chunks_sent={chunk_count} elapsed_ms={}",
                    StreamLifecycle::Interrupted.as_str(),
                    request_started.elapsed().as_millis()
                );
                yield Err(Status::internal(
                    "incomplete inference stream: missing final done chunk",
                ));
            }
        };

        Ok(Response::new(Box::pin(output)))
    }
}

fn extract_trace_id(metadata: &tonic::metadata::MetadataMap, request_id: u64) -> String {
    let trace_id = metadata
        .get("x-rex-trace-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    match trace_id {
        Some(value) => value.to_string(),
        None => format!("request-{request_id}"),
    }
}

fn cache_bypass_from_env() -> bool {
    let value = env::var("REX_CACHE_BYPASS").unwrap_or_default();
    value == "1" || value.eq_ignore_ascii_case("true")
}

fn format_cache_status(status: CacheStatus) -> &'static str {
    match status {
        CacheStatus::Hit => "hit",
        CacheStatus::MissStored => "miss_stored",
        CacheStatus::Bypass => "bypass",
    }
}

fn format_behavior_decision(decision: &BehaviorDecision) -> &'static str {
    match decision {
        BehaviorDecision::Allow => "allow",
        BehaviorDecision::Suppress { .. } => "suppress",
    }
}

#[derive(Debug, Clone)]
struct PromptDirectives {
    diagnostics_hint: Option<String>,
    cache_bypass: bool,
    behavior_snapshot: BehaviorSnapshot,
}

impl PromptDirectives {
    fn from_prompt(prompt: &str) -> Self {
        let mut diagnostics_hint = None;
        let mut cache_bypass = false;
        let mut behavior_snapshot = BehaviorSnapshot::default();
        for line in prompt.lines() {
            if let Some(value) = line
                .strip_prefix("[[diag:")
                .and_then(|text| text.strip_suffix("]]"))
            {
                let normalized = value.trim();
                if !normalized.is_empty() {
                    diagnostics_hint = Some(normalized.to_string());
                }
                continue;
            }
            if line.trim() == "[[cache:bypass]]" {
                cache_bypass = true;
                continue;
            }
            if line.trim() == "[[behavior:focused]]" {
                behavior_snapshot.typing_cadence_cpm = 500;
                behavior_snapshot.pause_events_last_minute = 0;
            }
        }
        Self {
            diagnostics_hint,
            cache_bypass,
            behavior_snapshot,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        extract_trace_id, format_behavior_decision, format_cache_status, PromptDirectives,
        RexDaemonService,
    };
    use crate::adapters::{MissingDoneMockRuntime, MockInferenceRuntime};
    use crate::l1_cache::L1Key;
    use crate::plugins::{BehaviorDecision, CacheStatus};
    use crate::policy::{PolicyEngine, ResponseCache};
    use futures::StreamExt;
    use rex_proto::rex::v1::rex_service_server::RexService;
    use rex_proto::rex::v1::{StreamInferenceRequest, StreamInferenceResponse};
    use std::sync::{Arc, Mutex};
    use std::time::Instant;
    use tonic::Request;

    #[test]
    fn stream_chunks_end_with_done_marker() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime should build");
        let chunks = runtime.block_on(RexDaemonService::build_inference_chunks("ping"));
        assert!(chunks.len() >= 3);

        let first = chunks[0].as_ref().expect("first chunk should be ok");
        assert!(!first.done);
        assert!(!first.text.is_empty());

        for (index, chunk) in chunks.iter().enumerate() {
            let chunk = chunk.as_ref().expect("chunk should be ok");
            assert_eq!(chunk.index, index as u64);
            if index < chunks.len() - 1 {
                assert!(!chunk.done);
                assert!(!chunk.text.is_empty());
            }
        }

        let last = chunks[chunks.len() - 1]
            .as_ref()
            .expect("last chunk should be ok");
        assert!(last.done);
        assert_eq!(last.text, "");
    }

    #[test]
    fn prompt_directives_parse_cache_and_diagnostics() {
        let directives =
            PromptDirectives::from_prompt("[[diag: cargo test failed]]\n[[cache:bypass]]");
        assert_eq!(
            directives.diagnostics_hint.as_deref(),
            Some("cargo test failed")
        );
        assert!(directives.cache_bypass);
    }

    #[test]
    fn cache_status_format_is_stable() {
        assert_eq!(format_cache_status(CacheStatus::Hit), "hit");
        assert_eq!(format_cache_status(CacheStatus::Bypass), "bypass");
    }

    #[test]
    fn behavior_decision_format_is_stable() {
        assert_eq!(format_behavior_decision(&BehaviorDecision::Allow), "allow");
        assert_eq!(
            format_behavior_decision(&BehaviorDecision::Suppress { reason: "x" }),
            "suppress"
        );
    }

    #[test]
    fn trace_id_extraction_falls_back_to_request_id() {
        let metadata = tonic::metadata::MetadataMap::new();
        assert_eq!(extract_trace_id(&metadata, 42), "request-42");
    }

    #[tokio::test]
    async fn stream_emits_grpc_error_when_runtime_omits_done() {
        let svc = RexDaemonService::with_runtime(Instant::now(), Arc::new(MissingDoneMockRuntime));
        let req = Request::new(StreamInferenceRequest {
            prompt: "x".to_string(),
            ..Default::default()
        });
        let mut out = svc
            .stream_inference(req)
            .await
            .expect("stream starts")
            .into_inner();
        let first = out.next().await.expect("stream item").expect("ok chunk");
        assert!(!first.done);
        let err = out
            .next()
            .await
            .expect("second item")
            .expect_err("terminal grpc error");
        assert_eq!(err.code(), tonic::Code::Internal);
        assert!(err
            .message()
            .contains("incomplete inference stream: missing final done chunk"));
        assert!(out.next().await.is_none());
    }

    /// Records cache call ordering so the policy seam ordering rule
    /// (`pipeline resolution -> cache decision -> runtime`) can be asserted
    /// from a real `stream_inference` call.
    #[derive(Default)]
    struct OrderingCache {
        events: Mutex<Vec<String>>,
    }

    impl ResponseCache for OrderingCache {
        fn get(&self, key: &L1Key) -> Option<Vec<StreamInferenceResponse>> {
            self.events
                .lock()
                .expect("ordering cache mutex")
                .push(format!("get:{}", key.mode));
            None
        }

        fn put(&self, key: L1Key, _value: Vec<StreamInferenceResponse>) {
            self.events
                .lock()
                .expect("ordering cache mutex")
                .push(format!("put:{}", key.mode));
        }
    }

    fn drain_events(cache: &OrderingCache) -> Vec<String> {
        cache.events.lock().expect("ordering cache mutex").clone()
    }

    #[tokio::test]
    async fn cache_is_skipped_for_agent_mode() {
        let cache = Arc::new(OrderingCache::default());
        let svc = RexDaemonService::with_runtime_and_policy(
            Instant::now(),
            Arc::new(MockInferenceRuntime),
            PolicyEngine::new(cache.clone()),
        );
        let req = Request::new(StreamInferenceRequest {
            prompt: "skip-me".to_string(),
            mode: "agent".to_string(),
            ..Default::default()
        });
        let mut out = svc
            .stream_inference(req)
            .await
            .expect("stream starts")
            .into_inner();
        while out.next().await.is_some() {}
        assert!(
            drain_events(&cache).is_empty(),
            "agent mode must not consult the response cache"
        );
    }

    #[tokio::test]
    async fn cache_is_skipped_when_prompt_requests_bypass() {
        let cache = Arc::new(OrderingCache::default());
        let svc = RexDaemonService::with_runtime_and_policy(
            Instant::now(),
            Arc::new(MockInferenceRuntime),
            PolicyEngine::new(cache.clone()),
        );
        let req = Request::new(StreamInferenceRequest {
            prompt: "[[cache:bypass]]\nhello".to_string(),
            mode: "ask".to_string(),
            ..Default::default()
        });
        let mut out = svc
            .stream_inference(req)
            .await
            .expect("stream starts")
            .into_inner();
        while out.next().await.is_some() {}
        assert!(
            drain_events(&cache).is_empty(),
            "operator bypass must not consult the response cache"
        );
    }

    #[tokio::test]
    async fn ask_mode_consults_cache_then_stores() {
        let cache = Arc::new(OrderingCache::default());
        let svc = RexDaemonService::with_runtime_and_policy(
            Instant::now(),
            Arc::new(MockInferenceRuntime),
            PolicyEngine::new(cache.clone()),
        );
        let req = Request::new(StreamInferenceRequest {
            prompt: "hello cache".to_string(),
            mode: "ask".to_string(),
            ..Default::default()
        });
        let mut out = svc
            .stream_inference(req)
            .await
            .expect("stream starts")
            .into_inner();
        while out.next().await.is_some() {}
        let events = drain_events(&cache);
        assert_eq!(
            events,
            vec!["get:ask".to_string(), "put:ask".to_string()],
            "ask mode must look up the cache before storing the runtime result"
        );
    }
}

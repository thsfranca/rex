use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::Instant;

use async_stream::stream;
use futures::StreamExt;
use rex_proto::rex::v1::rex_service_server::RexService;
use rex_proto::rex::v1::{
    BrokerExecShellRequest, BrokerExecShellResponse, BrokerInferenceRequest,
    BrokerInferenceResponse, BrokerListDirRequest, BrokerListDirResponse, BrokerReadFileRequest,
    BrokerReadFileResponse, BrokerSavePlanRequest, BrokerSavePlanResponse, BrokerWriteFileRequest,
    BrokerWriteFileResponse, GetSystemStatusRequest, GetSystemStatusResponse,
    StreamInferenceRequest, StreamInferenceResponse,
};
use tokio::time::{sleep, Duration};
use tokio_stream::Stream;
use tonic::{Request, Response, Status};

use crate::adapters::InferenceRuntime;
#[cfg(test)]
use crate::adapters::MockInferenceRuntime;
use crate::adapters::{active_model_id_from_config, AdapterCapabilities};
use crate::approvals::{ApprovalContext, ApprovalDecision, ApprovalGate};
use crate::broker::{
    broker_exec_shell, broker_list_dir, broker_read_file, broker_save_plan, broker_write_file,
    BrokerError,
};
use crate::broker_inference::run_broker_inference;
use crate::domain::{StreamLifecycle, ACTIVE_MODEL_ID, DAEMON_VERSION};
use crate::l1_cache::{l1_cachable_responses, normalize_mode};
use crate::observability::{observability_from_settings, StreamEconomicsDraft};
use crate::otlp_metrics::TerminalOtlpContext;
use crate::plugins::{
    BehaviorDecision, BehaviorSnapshot, CacheStatus, ContextPipeline, ContextRequest,
};
use crate::policy::{CacheDecision, CacheDecisionState, PolicyEngine, PolicyRequest};
use crate::routing::decide_route;
use crate::settings;
use crate::sidecar_client::{
    connect_sidecar, map_sidecar_to_inference_chunks, run_turn_collect, run_turn_stream,
};
use crate::sidecar_config::parse_harness_only;
use crate::supervisor::{SharedSupervisor, SupervisorError};
use crate::turn_correlation::{
    build_turn_correlation, strip_extension_context_blocks, TurnCorrelation,
};

pub struct RexDaemonService {
    started_at: Instant,
    pipeline: Mutex<ContextPipeline>,
    runtime: Arc<dyn InferenceRuntime>,
    request_sequence: AtomicU64,
    policy: PolicyEngine,
    approval_gate: Arc<dyn ApprovalGate>,
    sidecar: SharedSupervisor,
    pub(crate) observability: Option<Arc<crate::observability::ObservabilityRuntime>>,
}

const STREAM_CHUNK_DELAY_MS: u64 = 35;

impl RexDaemonService {
    /// Full-component constructor: inject a custom `PolicyEngine` and
    /// `ApprovalGate` for tests that need to observe cache call ordering or
    /// exercise non-`Allow` approval outcomes (R007 + R008 / ADR 0009).
    pub fn with_components(
        started_at: Instant,
        runtime: Arc<dyn InferenceRuntime>,
        policy: PolicyEngine,
        approval_gate: Arc<dyn ApprovalGate>,
        sidecar: SharedSupervisor,
    ) -> Self {
        Self {
            started_at,
            pipeline: Mutex::new(ContextPipeline::production_default()),
            runtime,
            request_sequence: AtomicU64::new(1),
            policy,
            approval_gate,
            sidecar,
            observability: observability_from_settings(),
        }
    }

    async fn resolve_inference_chunks(
        &self,
        effective_prompt: &str,
        mode: &str,
        model: &str,
        inference_runtime: &str,
        correlation: &TurnCorrelation,
    ) -> Result<Vec<Result<StreamInferenceResponse, Status>>, Status> {
        if parse_harness_only().is_some() || !self.sidecar.config().enabled {
            return Ok(self.runtime.build_chunks(effective_prompt).await);
        }
        self.sidecar
            .ensure_running()
            .await
            .map_err(|err| sidecar_error_to_status(&err, self.sidecar.config().required))?;
        let socket = self.sidecar.config().socket_path.clone();
        let mut client = connect_sidecar(&socket)
            .await
            .map_err(|e| Status::unavailable(format!("sidecar connect failed: {e}")))?;
        let sidecar_chunks =
            run_turn_collect(&mut client, effective_prompt, mode, model, correlation)
                .await
                .map_err(|e| Status::internal(format!("sidecar RunTurn failed: {e}")))?;
        println!(
            "stream.sidecar=ok inference_runtime=sidecar sidecar_socket={socket} mode={} turn_id={}",
            normalize_mode(mode),
            correlation.turn_id
        );
        let _ = inference_runtime;
        Ok(map_sidecar_to_inference_chunks(sidecar_chunks))
    }

    fn sidecar_live_stream_active(&self) -> bool {
        parse_harness_only().is_none() && self.sidecar.config().enabled
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
    async fn broker_exec_shell(
        &self,
        request: Request<BrokerExecShellRequest>,
    ) -> Result<Response<BrokerExecShellResponse>, Status> {
        let inner = request.into_inner();
        let mode = normalize_mode(&inner.mode);
        let command = inner.command;
        println!(
            "broker.access_policy=evaluate capability=exec.shell mode={mode} command={command}"
        );
        match broker_exec_shell(&command, &mode) {
            Ok(result) => {
                println!("broker.access_policy=allow capability=exec.shell mode={mode}");
                Ok(Response::new(BrokerExecShellResponse {
                    ok: true,
                    stdout: result.stdout,
                    stderr: result.stderr,
                    error: String::new(),
                }))
            }
            Err(err) => {
                log_broker_access_policy_deny("exec.shell", &mode, &command, &err);
                Ok(Response::new(BrokerExecShellResponse {
                    ok: false,
                    stdout: String::new(),
                    stderr: String::new(),
                    error: err.to_string(),
                }))
            }
        }
    }

    async fn broker_save_plan(
        &self,
        request: Request<BrokerSavePlanRequest>,
    ) -> Result<Response<BrokerSavePlanResponse>, Status> {
        let inner = request.into_inner();
        let mode = normalize_mode(&inner.mode);
        let path = inner.path;
        println!("broker.access_policy=evaluate capability=plan.save mode={mode} path={path}");
        match broker_save_plan(&path, &inner.content, &mode) {
            Ok(()) => {
                println!("broker.access_policy=allow capability=plan.save mode={mode} path={path}");
                Ok(Response::new(BrokerSavePlanResponse {
                    ok: true,
                    error: String::new(),
                }))
            }
            Err(err) => {
                log_broker_access_policy_deny("plan.save", &mode, &path, &err);
                Ok(Response::new(BrokerSavePlanResponse {
                    ok: false,
                    error: err.to_string(),
                }))
            }
        }
    }

    async fn broker_write_file(
        &self,
        request: Request<BrokerWriteFileRequest>,
    ) -> Result<Response<BrokerWriteFileResponse>, Status> {
        let inner = request.into_inner();
        let mode = normalize_mode(&inner.mode);
        let path = inner.path;
        println!("broker.access_policy=evaluate capability=fs.write mode={mode} path={path}");
        match broker_write_file(&path, &inner.content, &mode) {
            Ok(()) => {
                println!("broker.access_policy=allow capability=fs.write mode={mode} path={path}");
                Ok(Response::new(BrokerWriteFileResponse {
                    ok: true,
                    error: String::new(),
                }))
            }
            Err(err) => {
                log_broker_access_policy_deny("fs.write", &mode, &path, &err);
                Ok(Response::new(BrokerWriteFileResponse {
                    ok: false,
                    error: err.to_string(),
                }))
            }
        }
    }

    async fn broker_list_dir(
        &self,
        request: Request<BrokerListDirRequest>,
    ) -> Result<Response<BrokerListDirResponse>, Status> {
        let inner = request.into_inner();
        let mode = normalize_mode(&inner.mode);
        let path = inner.path;
        println!("broker.access_policy=evaluate capability=fs.list mode={mode} path={path}");
        match broker_list_dir(&path, &mode) {
            Ok(entries) => {
                println!("broker.access_policy=allow capability=fs.list mode={mode} path={path}");
                Ok(Response::new(BrokerListDirResponse {
                    ok: true,
                    entries: entries
                        .into_iter()
                        .map(|entry| rex_proto::rex::v1::DirEntry {
                            name: entry.name,
                            is_dir: entry.is_dir,
                        })
                        .collect(),
                    error: String::new(),
                }))
            }
            Err(err) => {
                log_broker_access_policy_deny("fs.list", &mode, &path, &err);
                Ok(Response::new(BrokerListDirResponse {
                    ok: false,
                    entries: Vec::new(),
                    error: err.to_string(),
                }))
            }
        }
    }

    async fn broker_read_file(
        &self,
        request: Request<BrokerReadFileRequest>,
    ) -> Result<Response<BrokerReadFileResponse>, Status> {
        let inner = request.into_inner();
        let mode = normalize_mode(&inner.mode);
        let path = inner.path;
        println!("broker.access_policy=evaluate capability=fs.read mode={mode} path={path}");
        match broker_read_file(&path, &mode) {
            Ok(content) => {
                println!("broker.access_policy=allow capability=fs.read mode={mode} path={path}");
                Ok(Response::new(BrokerReadFileResponse {
                    ok: true,
                    content,
                    error: String::new(),
                }))
            }
            Err(err) => {
                log_broker_access_policy_deny("fs.read", &mode, &path, &err);
                Ok(Response::new(BrokerReadFileResponse {
                    ok: false,
                    content: String::new(),
                    error: err.to_string(),
                }))
            }
        }
    }

    async fn broker_inference(
        &self,
        request: Request<BrokerInferenceRequest>,
    ) -> Result<Response<BrokerInferenceResponse>, Status> {
        let turn_id = extract_turn_id(request.metadata());
        let inner = request.into_inner();
        let mode = normalize_mode(&inner.mode);
        let tool_count = inner.tools.len();
        let message_count = if inner.messages.is_empty() {
            usize::from(!inner.prompt.trim().is_empty())
        } else {
            inner.messages.len()
        };
        println!(
            "broker.inference=requested turn_id={turn_id} mode={mode} messages={message_count} tools={tool_count}"
        );
        match run_broker_inference(&inner).await {
            Ok(response) => {
                let protocol = response.protocol;
                let tool_calls = response.tool_calls.len();
                if response.ok {
                    println!(
                        "broker.inference=ok turn_id={turn_id} mode={mode} protocol={protocol} tool_calls={tool_calls}"
                    );
                } else {
                    println!(
                        "broker.inference=error turn_id={turn_id} mode={mode} protocol={protocol} error={}",
                        response.error
                    );
                }
                Ok(Response::new(response))
            }
            Err(status) => {
                println!(
                    "broker.inference=error turn_id={turn_id} mode={mode} error={}",
                    status.message()
                );
                Err(status)
            }
        }
    }

    async fn get_system_status(
        &self,
        _request: Request<GetSystemStatusRequest>,
    ) -> Result<Response<GetSystemStatusResponse>, Status> {
        Ok(Response::new(GetSystemStatusResponse {
            daemon_version: DAEMON_VERSION.to_string(),
            uptime_seconds: self.started_at.elapsed().as_secs(),
            active_model_id: active_model_id_from_config(),
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
        let decision_id = format!("dec-{request_id}");
        let trace_id = extract_trace_id(request.metadata(), request_id);
        ensure_workspace_configured()?;
        let inner = request.into_inner();
        let mut prompt = inner.prompt;
        if let Some(hints) = inner.client_hints.as_ref() {
            if !hints.active_file_path.is_empty() {
                println!(
                    "stream.request_id={request_id} trace_id={trace_id} client_hints.active_file={}",
                    hints.active_file_path
                );
            }
            if !hints.active_file_path.is_empty()
                || !hints.language_id.is_empty()
                || !hints.selection_text.is_empty()
            {
                let (stripped, applied) = strip_extension_context_blocks(&prompt);
                if applied {
                    prompt = stripped;
                }
            }
        }
        let model = inner.model;
        let mode = inner.mode;
        let route = decide_route(&mode, &model);
        let runtime_kind = route.runtime;
        let inference_runtime = runtime_kind.log_label();
        let adapter_capabilities = AdapterCapabilities::for_runtime(runtime_kind);
        let directives = PromptDirectives::from_prompt(&prompt);
        let active_file_path = inner
            .client_hints
            .as_ref()
            .filter(|h| !h.active_file_path.is_empty())
            .map(|h| h.active_file_path.clone());
        let context_request = ContextRequest {
            prompt: prompt.clone(),
            diagnostics_hint: directives.diagnostics_hint.clone(),
            cache_bypass: directives.cache_bypass || cache_bypass_from_config(),
            behavior_snapshot: directives.behavior_snapshot,
            retrieve_off: directives.retrieve_off,
            active_file_path,
        };
        let pipeline_result = self
            .pipeline
            .lock()
            .expect("context pipeline mutex should not be poisoned")
            .prepare(&context_request, adapter_capabilities);
        let correlation = build_turn_correlation(
            request_id,
            &pipeline_result.injected_context,
            pipeline_result.metrics.retrieval.as_str(),
            pipeline_result.metrics.compression_strategy,
            pipeline_result.metrics.context_selected,
            pipeline_result.metrics.context_truncated,
        );
        if pipeline_result.c1_stripped {
            println!(
                "stream.request_id={request_id} trace_id={trace_id} turn_id={} context.strip=c1",
                correlation.turn_id
            );
        }
        let prompt_len = prompt.chars().count();
        let route_label = resolve_route_label_for(
            inference_runtime,
            parse_harness_only().is_some(),
            self.sidecar_live_stream_active(),
        );
        println!(
            "stream.request_id={request_id} trace_id={trace_id} turn_id={} context_revision={} inference_runtime={inference_runtime} route={route_label} decision_id={decision_id} stream.lifecycle={} prompt_len={prompt_len}",
            correlation.turn_id,
            correlation.context_revision,
            StreamLifecycle::Starting.as_str(),
        );
        println!(
            "stream.request_id={request_id} trace_id={trace_id} turn_id={} context_revision={} inference_runtime={inference_runtime} stream.metrics prompt_tokens={} context_tokens={} candidates={} selected={} truncated={} cache={} behavior={} retrieval={} compression_strategy={}",
            correlation.turn_id,
            correlation.context_revision,
            pipeline_result.metrics.prompt_tokens,
            pipeline_result.metrics.selected_context_tokens,
            pipeline_result.metrics.context_candidates,
            pipeline_result.metrics.context_selected,
            pipeline_result.metrics.context_truncated,
            format_cache_status(pipeline_result.metrics.cache_status),
            format_behavior_decision(&pipeline_result.metrics.behavior_decision),
            pipeline_result.metrics.retrieval.as_str(),
            pipeline_result.metrics.compression_strategy,
        );
        let cache_bypass = directives.cache_bypass || cache_bypass_from_config();
        let policy_request = PolicyRequest {
            runtime: runtime_kind,
            model: &model,
            mode: &mode,
            effective_prompt: &pipeline_result.effective_prompt,
            cache_bypass,
        };
        let decision = self.policy.decide(&policy_request);
        let mut approval_outcome_label: Option<String> = None;
        if normalize_mode(&mode) == "agent" {
            let approval_id = inner.approval_id.trim();
            let approval_ctx = ApprovalContext {
                mode: mode.clone(),
                runtime: policy_request.runtime,
                approval_id: if approval_id.is_empty() {
                    None
                } else {
                    Some(approval_id.to_string())
                },
            };
            let approval_outcome = self.approval_gate.check(&approval_ctx).await;
            let approval_label = format_approval_decision(&approval_outcome);
            approval_outcome_label = Some(approval_label.to_string());
            match &approval_outcome {
                ApprovalDecision::Allow => {
                    println!(
                        "stream.request_id={request_id} trace_id={trace_id} inference_runtime={inference_runtime} approval={approval_label}",
                    );
                }
                ApprovalDecision::Checkpoint { reason } => {
                    println!(
                        "stream.request_id={request_id} trace_id={trace_id} inference_runtime={inference_runtime} approval={approval_label} stream.lifecycle={} stream.event=approval_checkpoint reason={reason} elapsed_ms={}",
                        StreamLifecycle::Failed.as_str(),
                        request_started.elapsed().as_millis()
                    );
                    return Err(Status::failed_precondition(format!(
                        "agent execution checkpoint required: {reason}"
                    )));
                }
                ApprovalDecision::Deny { reason } => {
                    println!(
                        "stream.request_id={request_id} trace_id={trace_id} inference_runtime={inference_runtime} approval={approval_label} stream.lifecycle={} stream.event=approval_denied reason={reason} elapsed_ms={}",
                        StreamLifecycle::Failed.as_str(),
                        request_started.elapsed().as_millis()
                    );
                    return Err(Status::failed_precondition(format!(
                        "agent execution denied by approval gate: {reason}"
                    )));
                }
            }
        }
        let mut l1_state: Option<&'static str> = None;
        let effective_prompt = pipeline_result.effective_prompt.clone();
        let effective_model = model.clone();
        let correlation_for_sidecar = correlation.clone();
        let sidecar_live = self.sidecar_live_stream_active();
        let lookup_key = match &decision {
            CacheDecision::Lookup(key) => Some(key.clone()),
            _ => None,
        };
        let cached_chunks = lookup_key.as_ref().and_then(|key| self.policy.get(key));
        let use_sidecar_live = sidecar_live && cached_chunks.is_none();
        let chunks: Vec<Result<StreamInferenceResponse, Status>> =
            if let Some(cached) = cached_chunks {
                l1_state = Some("hit");
                cached.into_iter().map(Ok).collect()
            } else if use_sidecar_live {
                if lookup_key.is_some() {
                    l1_state = Some("miss");
                }
                Vec::new()
            } else if let Some(key) = lookup_key {
                l1_state = Some("miss");
                let built = self
                    .resolve_inference_chunks(
                        &effective_prompt,
                        &mode,
                        &effective_model,
                        inference_runtime,
                        &correlation_for_sidecar,
                    )
                    .await?;
                if let Some(to_store) = l1_cachable_responses(&built) {
                    self.policy.put(key, to_store);
                }
                built
            } else {
                self.resolve_inference_chunks(
                    &effective_prompt,
                    &mode,
                    &effective_model,
                    inference_runtime,
                    &correlation_for_sidecar,
                )
                .await?
            };
        let cache_decision_state =
            CacheDecisionState::from_outcome(&decision, matches!(l1_state, Some("hit")));
        let log_model = if model.trim().is_empty() {
            ACTIVE_MODEL_ID
        } else {
            model.trim()
        };
        let log_mode = normalize_mode(&mode);
        if let Some(state) = l1_state {
            println!(
                "stream.request_id={request_id} trace_id={trace_id} turn_id={} inference_runtime={inference_runtime} l1_cache={state} model={log_model} mode={log_mode}",
                correlation.turn_id,
            );
        }
        println!(
            "stream.request_id={request_id} trace_id={trace_id} turn_id={} inference_runtime={inference_runtime} cache_decision={} model={log_model} mode={log_mode}",
            correlation.turn_id,
            cache_decision_state.label(),
        );
        if !use_sidecar_live {
            println!(
                "stream.request_id={request_id} trace_id={trace_id} turn_id={} inference_runtime={inference_runtime} stream.lifecycle={} chunk_count={}",
                correlation.turn_id,
                StreamLifecycle::Streaming.as_str(),
                chunks.len()
            );
        } else {
            println!(
                "stream.request_id={request_id} trace_id={trace_id} turn_id={} inference_runtime={inference_runtime} stream.lifecycle={} chunk_count=live_sidecar",
                correlation.turn_id,
                StreamLifecycle::Streaming.as_str(),
            );
        }
        let sidecar = self.sidecar.clone();
        let sidecar_socket = self.sidecar.config().socket_path.clone();
        let turn_id_for_stream = correlation.turn_id.clone();
        let observability = self.observability.clone();
        let economics_draft = observability.as_ref().map(|obs| StreamEconomicsDraft {
            snapshot_id: obs.snapshot_id().to_string(),
            request_id,
            trace_id: trace_id.clone(),
            turn_id: correlation.turn_id.clone(),
            route: route_label.clone(),
            cache_decision: cache_decision_state.label().to_string(),
            decision_id: decision_id.clone(),
            inference_runtime: inference_runtime.to_string(),
            mode: log_mode.to_string(),
            model: log_model.to_string(),
            metrics: pipeline_result.metrics.clone(),
            approval_outcome: approval_outcome_label.clone(),
        });
        let output = stream! {
            let mut chunk_count: u64 = 0;
            let mut done_seen = false;
            let mut ttft_ms: Option<u64> = None;
            if use_sidecar_live {
                sidecar
                    .ensure_running()
                    .await
                    .map_err(|err| sidecar_error_to_status(&err, sidecar.config().required))?;
                let mut client = connect_sidecar(&sidecar_socket)
                    .await
                    .map_err(|e| Status::unavailable(format!("sidecar connect failed: {e}")))?;
                let mut sidecar_stream = run_turn_stream(
                    &mut client,
                    &effective_prompt,
                    &mode,
                    &effective_model,
                    &correlation_for_sidecar,
                )
                .await
                .map_err(|e| Status::internal(format!("sidecar RunTurn failed: {e}")))?;
                println!(
                    "stream.sidecar=ok inference_runtime=sidecar sidecar_socket={sidecar_socket} mode={log_mode} turn_id={turn_id_for_stream}",
                );
                while let Some(chunk) = sidecar_stream.next().await {
                    match chunk {
                        Ok(chunk) => {
                            chunk_count += 1;
                            if chunk_count == 1 {
                                ttft_ms = Some(request_started.elapsed().as_millis() as u64);
                                println!(
                                    "stream.request_id={request_id} trace_id={trace_id} turn_id={turn_id_for_stream} inference_runtime={inference_runtime} stream.event=first_chunk index={} done={}",
                                    chunk.index,
                                    chunk.done
                                );
                            }
                            if chunk.done {
                                done_seen = true;
                            }
                            yield Ok(chunk);
                        }
                        Err(err) => {
                            let elapsed_ms = request_started.elapsed().as_millis() as u64;
                            println!(
                                "stream.request_id={request_id} trace_id={trace_id} turn_id={turn_id_for_stream} inference_runtime={inference_runtime} stream.lifecycle={} stream.event=error stream.terminal=grpc_error grpc_code={} message={} elapsed_ms={elapsed_ms}",
                                StreamLifecycle::Failed.as_str(),
                                err.code() as i32,
                                err.message(),
                            );
                            if let (Some(obs), Some(draft)) =
                                (observability.as_ref(), economics_draft.clone())
                            {
                                let ctx = terminal_otlp_ctx(ttft_ms, &draft, Some("grpc_error"));
                                obs.record_terminal_async(
                                    draft,
                                    "grpc_error",
                                    elapsed_ms,
                                    chunk_count,
                                    ctx,
                                );
                            }
                            yield Err(err);
                            return;
                        }
                    }
                }
            } else {
                for chunk in chunks {
                    match chunk {
                        Ok(chunk) => {
                            chunk_count += 1;
                            if chunk_count == 1 {
                                ttft_ms = Some(request_started.elapsed().as_millis() as u64);
                                println!(
                                    "stream.request_id={request_id} trace_id={trace_id} turn_id={turn_id_for_stream} inference_runtime={inference_runtime} stream.event=first_chunk index={} done={}",
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
                            let elapsed_ms = request_started.elapsed().as_millis() as u64;
                            println!(
                                "stream.request_id={request_id} trace_id={trace_id} turn_id={turn_id_for_stream} inference_runtime={inference_runtime} stream.lifecycle={} stream.event=error stream.terminal=grpc_error grpc_code={} message={} elapsed_ms={elapsed_ms}",
                                StreamLifecycle::Failed.as_str(),
                                err.code() as i32,
                                err.message(),
                            );
                            if let (Some(obs), Some(draft)) =
                                (observability.as_ref(), economics_draft.clone())
                            {
                                let ctx = terminal_otlp_ctx(ttft_ms, &draft, Some("grpc_error"));
                                obs.record_terminal_async(
                                    draft,
                                    "grpc_error",
                                    elapsed_ms,
                                    chunk_count,
                                    ctx,
                                );
                            }
                            yield Err(err);
                            return;
                        }
                    }
                }
            }
            let elapsed_ms = request_started.elapsed().as_millis() as u64;
            if done_seen {
                println!(
                    "stream.request_id={request_id} trace_id={trace_id} turn_id={turn_id_for_stream} inference_runtime={inference_runtime} stream.lifecycle={} stream.terminal=done chunks_sent={chunk_count} elapsed_ms={elapsed_ms}",
                    StreamLifecycle::Completed.as_str(),
                );
                if let (Some(obs), Some(draft)) =
                    (observability.as_ref(), economics_draft.clone())
                {
                    let ctx = terminal_otlp_ctx(ttft_ms, &draft, None);
                    obs.record_terminal_async(draft, "done", elapsed_ms, chunk_count, ctx);
                }
            } else {
                println!(
                    "stream.request_id={request_id} trace_id={trace_id} turn_id={turn_id_for_stream} inference_runtime={inference_runtime} stream.lifecycle={} stream.event=incomplete stream.terminal=missing_done chunks_sent={chunk_count} elapsed_ms={elapsed_ms}",
                    StreamLifecycle::Interrupted.as_str(),
                );
                if let (Some(obs), Some(draft)) =
                    (observability.as_ref(), economics_draft.clone())
                {
                    let ctx = terminal_otlp_ctx(ttft_ms, &draft, Some("missing_done"));
                    obs.record_terminal_async(
                        draft,
                        "missing_done",
                        elapsed_ms,
                        chunk_count,
                        ctx,
                    );
                }
                yield Err(Status::internal(
                    "incomplete inference stream: missing final done chunk",
                ));
            }
        };

        Ok(Response::new(Box::pin(output)))
    }
}

#[allow(clippy::result_large_err)]
fn ensure_workspace_configured() -> Result<(), Status> {
    let loaded = settings::get();
    if !loaded
        .workspace_indexer_mode()
        .trim()
        .eq_ignore_ascii_case("workspace")
    {
        return Ok(());
    }
    loaded.resolve_workspace_root().map(|_| ()).map_err(|_| {
        eprintln!("workspace.error=not_configured");
        Status::failed_precondition(
            "workspace root not configured (set workspace.root in .rex/config.json)",
        )
    })
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

fn log_broker_access_policy_deny(capability: &str, mode: &str, subject: &str, err: &BrokerError) {
    if let BrokerError::PolicyDenied { code, message } = err {
        println!(
            "broker.access_policy=deny capability={capability} mode={mode} code={code} subject={subject} error={message}"
        );
    } else {
        println!(
            "broker.access_policy=deny capability={capability} mode={mode} subject={subject} error={err}"
        );
    }
}

fn extract_turn_id(metadata: &tonic::metadata::MetadataMap) -> String {
    metadata
        .get("x-rex-turn-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("")
        .to_string()
}

fn cache_bypass_from_config() -> bool {
    crate::settings::get().cache_bypass()
}

fn format_cache_status(status: CacheStatus) -> &'static str {
    match status {
        CacheStatus::Hit => "hit",
        CacheStatus::MissStored => "miss_stored",
        CacheStatus::Bypass => "bypass",
    }
}

pub(crate) fn resolve_route_label_for(
    inference_runtime: &str,
    harness_only: bool,
    sidecar_product_path: bool,
) -> String {
    if harness_only {
        return format!("harness_direct+{inference_runtime}");
    }
    if sidecar_product_path {
        return format!("sidecar+{inference_runtime}");
    }
    format!("daemon_direct+{inference_runtime}")
}

fn sidecar_error_to_status(err: &SupervisorError, required: bool) -> Status {
    let message = err.to_string();
    if required {
        Status::failed_precondition(format!("sidecar required but unavailable: {message}"))
    } else {
        Status::unavailable(format!("sidecar unavailable: {message}"))
    }
}

fn format_behavior_decision(decision: &BehaviorDecision) -> &'static str {
    match decision {
        BehaviorDecision::Allow => "allow",
        BehaviorDecision::Suppress { .. } => "suppress",
    }
}

fn terminal_otlp_ctx(
    ttft_ms: Option<u64>,
    draft: &StreamEconomicsDraft,
    error_type: Option<&str>,
) -> TerminalOtlpContext {
    TerminalOtlpContext {
        ttft_ms,
        approval_outcome: draft.approval_outcome.clone(),
        error_type: error_type.map(str::to_string),
        broker_inference_outcome: None,
        load_duration_ms: None,
    }
}

fn format_approval_decision(decision: &ApprovalDecision) -> &'static str {
    match decision {
        ApprovalDecision::Allow => "allow",
        ApprovalDecision::Deny { .. } => "deny",
        ApprovalDecision::Checkpoint { .. } => "checkpoint",
    }
}

#[derive(Debug, Clone)]
struct PromptDirectives {
    diagnostics_hint: Option<String>,
    cache_bypass: bool,
    behavior_snapshot: BehaviorSnapshot,
    retrieve_off: bool,
}

impl PromptDirectives {
    fn from_prompt(prompt: &str) -> Self {
        let mut diagnostics_hint = None;
        let mut cache_bypass = false;
        let mut behavior_snapshot = BehaviorSnapshot::default();
        let mut retrieve_off = false;
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
                continue;
            }
            if line.trim() == "[[retrieve:off]]" {
                retrieve_off = true;
            }
        }
        Self {
            diagnostics_hint,
            cache_bypass,
            behavior_snapshot,
            retrieve_off,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        extract_trace_id, extract_turn_id, format_approval_decision, format_behavior_decision,
        format_cache_status, resolve_route_label_for, PromptDirectives, RexDaemonService,
    };
    use crate::adapters::{MissingDoneMockRuntime, MockInferenceRuntime};
    use crate::sidecar_config::SidecarConfig;
    use crate::supervisor::{SharedSupervisor, SidecarSupervisor};

    fn disabled_sidecar() -> SharedSupervisor {
        std::sync::Arc::new(SidecarSupervisor::new(SidecarConfig {
            enabled: false,
            required: false,
            binary: PathBuf::from("rex-sidecar-stub"),
            socket_path: "/tmp/rex-test-sidecar.sock".to_string(),
        }))
    }
    use crate::approvals::{ApprovalContext, ApprovalDecision, ApprovalGate};
    use crate::l1_cache::L1Key;
    use crate::plugins::{BehaviorDecision, CacheStatus};
    use crate::policy::{PolicyEngine, ResponseCache};
    use crate::settings;
    use futures::StreamExt;
    use rex_config::LoadedConfig;
    use rex_proto::rex::v1::rex_service_server::RexService;
    use rex_proto::rex::v1::{StreamInferenceRequest, StreamInferenceResponse};
    use serial_test::serial;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;
    use tonic::Request;

    fn init_stream_test_settings() {
        settings::reset_for_test();
        let mut cfg = rex_config::RexConfig::defaults();
        cfg.workspace.allow_cwd_fallback = Some(true);
        cfg.sidecars.harness = Some("direct".to_string());
        cfg.sidecars.required = Some(false);
        if let Some(entry) = cfg.sidecars.list.first_mut() {
            entry.enabled = false;
        }
        settings::init_for_test(Arc::new(LoadedConfig {
            rex_root: std::path::PathBuf::from("/tmp/rex-test"),
            global_path: None,
            project_path: None,
            effective: cfg,
        }));
    }

    fn init_unconfigured_workspace_settings() {
        settings::reset_for_test();
        let mut cfg = rex_config::RexConfig::defaults();
        cfg.workspace.root = String::new();
        cfg.workspace.allow_cwd_fallback = None;
        cfg.workspace.indexer = "workspace".to_string();
        cfg.sidecars.harness = Some("direct".to_string());
        cfg.sidecars.required = Some(false);
        if let Some(entry) = cfg.sidecars.list.first_mut() {
            entry.enabled = false;
        }
        settings::init_for_test(Arc::new(LoadedConfig {
            rex_root: std::path::PathBuf::from("/tmp/rex-test"),
            global_path: None,
            project_path: None,
            effective: cfg,
        }));
    }

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
        assert_eq!(format_approval_decision(&ApprovalDecision::Allow), "allow");
        assert_eq!(
            format_approval_decision(&ApprovalDecision::Deny {
                reason: "x".to_string(),
            }),
            "deny"
        );
        assert_eq!(
            format_approval_decision(&ApprovalDecision::Checkpoint {
                reason: "y".to_string(),
            }),
            "checkpoint"
        );
    }

    #[test]
    fn resolve_route_label_sidecar_and_harness_modes() {
        assert_eq!(
            resolve_route_label_for("http-openai-compat", false, true),
            "sidecar+http-openai-compat"
        );
        assert_eq!(
            resolve_route_label_for("mock", true, false),
            "harness_direct+mock"
        );
        assert_eq!(
            resolve_route_label_for("mock", true, true),
            "harness_direct+mock"
        );
        assert_eq!(
            resolve_route_label_for("mock", false, false),
            "daemon_direct+mock"
        );
    }

    #[test]
    fn trace_id_extraction_falls_back_to_request_id() {
        let metadata = tonic::metadata::MetadataMap::new();
        assert_eq!(extract_trace_id(&metadata, 42), "request-42");
    }

    #[test]
    fn extract_turn_id_reads_metadata() {
        let mut metadata = tonic::metadata::MetadataMap::new();
        metadata.insert("x-rex-turn-id", "turn-9".parse().unwrap());
        assert_eq!(extract_turn_id(&metadata), "turn-9");
        assert_eq!(extract_turn_id(&tonic::metadata::MetadataMap::new()), "");
    }

    #[tokio::test]
    #[serial]
    async fn stream_fails_when_workspace_unconfigured() {
        init_unconfigured_workspace_settings();
        let svc = RexDaemonService::with_components(
            Instant::now(),
            Arc::new(MockInferenceRuntime),
            PolicyEngine::with_default_layers(),
            Arc::new(crate::approvals::AlwaysAllow),
            disabled_sidecar(),
        );
        let req = Request::new(StreamInferenceRequest {
            prompt: "hello".to_string(),
            mode: "ask".to_string(),
            ..Default::default()
        });
        let err = match svc.stream_inference(req).await {
            Ok(_) => panic!("missing workspace.root must fail closed"),
            Err(e) => e,
        };
        assert_eq!(err.code(), tonic::Code::FailedPrecondition);
        assert!(
            err.message().contains("workspace root not configured"),
            "message: {}",
            err.message()
        );
    }

    #[tokio::test]
    #[serial]
    async fn stream_emits_grpc_error_when_runtime_omits_done() {
        init_stream_test_settings();
        let svc = RexDaemonService::with_components(
            Instant::now(),
            Arc::new(MissingDoneMockRuntime),
            PolicyEngine::with_default_layers(),
            Arc::new(crate::approvals::AlwaysAllow),
            disabled_sidecar(),
        );
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
    #[serial]
    async fn cache_is_skipped_for_agent_mode() {
        init_stream_test_settings();
        let cache = Arc::new(OrderingCache::default());
        let svc = RexDaemonService::with_components(
            Instant::now(),
            Arc::new(MockInferenceRuntime),
            PolicyEngine::new(cache.clone()),
            Arc::new(crate::approvals::AlwaysAllow),
            disabled_sidecar(),
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
    #[serial]
    async fn cache_is_skipped_when_prompt_requests_bypass() {
        init_stream_test_settings();
        let cache = Arc::new(OrderingCache::default());
        let svc = RexDaemonService::with_components(
            Instant::now(),
            Arc::new(MockInferenceRuntime),
            PolicyEngine::new(cache.clone()),
            Arc::new(crate::approvals::AlwaysAllow),
            disabled_sidecar(),
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

    /// Records each gate invocation so tests can assert the gate is consulted
    /// only for `agent` mode and that its decision propagates correctly.
    struct RecordingGate {
        decision: ApprovalDecision,
        calls: Mutex<Vec<String>>,
    }

    impl RecordingGate {
        fn new(decision: ApprovalDecision) -> Self {
            Self {
                decision,
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    #[tonic::async_trait]
    impl ApprovalGate for RecordingGate {
        async fn check(&self, ctx: &ApprovalContext) -> ApprovalDecision {
            self.calls
                .lock()
                .expect("gate calls mutex")
                .push(ctx.mode.clone());
            self.decision.clone()
        }
    }

    #[tokio::test]
    #[serial]
    async fn approval_gate_is_not_consulted_for_ask_mode() {
        init_stream_test_settings();
        let gate = Arc::new(RecordingGate::new(ApprovalDecision::Allow));
        let svc = RexDaemonService::with_components(
            Instant::now(),
            Arc::new(MockInferenceRuntime),
            PolicyEngine::with_default_layers(),
            gate.clone(),
            disabled_sidecar(),
        );
        let req = Request::new(StreamInferenceRequest {
            prompt: "ask path".to_string(),
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
            gate.calls.lock().expect("gate calls mutex").is_empty(),
            "ask mode must never consult the approval gate (ADR 0009)"
        );
    }

    #[tokio::test]
    #[serial]
    async fn approval_gate_is_consulted_for_agent_mode_and_allow_passes() {
        init_stream_test_settings();
        let gate = Arc::new(RecordingGate::new(ApprovalDecision::Allow));
        let svc = RexDaemonService::with_components(
            Instant::now(),
            Arc::new(MockInferenceRuntime),
            PolicyEngine::with_default_layers(),
            gate.clone(),
            disabled_sidecar(),
        );
        let req = Request::new(StreamInferenceRequest {
            prompt: "agent path".to_string(),
            mode: "agent".to_string(),
            ..Default::default()
        });
        let mut out = svc
            .stream_inference(req)
            .await
            .expect("stream starts when gate allows")
            .into_inner();
        let mut last_done = false;
        while let Some(chunk) = out.next().await {
            let chunk = chunk.expect("ok chunk");
            last_done = chunk.done;
        }
        assert!(last_done, "stream should reach the terminal done chunk");
        let calls = gate.calls.lock().expect("gate calls mutex").clone();
        assert_eq!(calls, vec!["agent".to_string()]);
    }

    #[tokio::test]
    #[serial]
    async fn approval_gate_deny_returns_typed_grpc_error_for_agent_mode() {
        init_stream_test_settings();
        let gate = Arc::new(RecordingGate::new(ApprovalDecision::Deny {
            reason: "manual approval required".to_string(),
        }));
        let svc = RexDaemonService::with_components(
            Instant::now(),
            Arc::new(MockInferenceRuntime),
            PolicyEngine::with_default_layers(),
            gate,
            disabled_sidecar(),
        );
        let req = Request::new(StreamInferenceRequest {
            prompt: "agent path".to_string(),
            mode: "agent".to_string(),
            ..Default::default()
        });
        let err = match svc.stream_inference(req).await {
            Ok(_) => panic!("deny must surface as a typed gRPC error before streaming"),
            Err(e) => e,
        };
        assert_eq!(err.code(), tonic::Code::FailedPrecondition);
        assert!(
            err.message().contains("manual approval required"),
            "error message must include gate-supplied reason: {}",
            err.message()
        );
    }

    #[tokio::test]
    #[serial]
    async fn approval_gate_checkpoint_returns_failed_precondition() {
        init_stream_test_settings();
        let gate = Arc::new(RecordingGate::new(ApprovalDecision::Checkpoint {
            reason: "future tool gate".to_string(),
        }));
        let svc = RexDaemonService::with_components(
            Instant::now(),
            Arc::new(MockInferenceRuntime),
            PolicyEngine::with_default_layers(),
            gate,
            disabled_sidecar(),
        );
        let req = Request::new(StreamInferenceRequest {
            prompt: "agent path".to_string(),
            mode: "agent".to_string(),
            ..Default::default()
        });
        let result = svc.stream_inference(req).await;
        assert!(result.is_err(), "checkpoint must block stream start");
        let err = result.err().expect("status error");
        assert!(
            err.message().contains("checkpoint required"),
            "unexpected message: {}",
            err.message()
        );
    }

    #[tokio::test]
    #[serial]
    async fn ask_mode_consults_cache_then_stores() {
        init_stream_test_settings();
        let cache = Arc::new(OrderingCache::default());
        let svc = RexDaemonService::with_components(
            Instant::now(),
            Arc::new(MockInferenceRuntime),
            PolicyEngine::new(cache.clone()),
            Arc::new(crate::approvals::AlwaysAllow),
            disabled_sidecar(),
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

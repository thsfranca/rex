use std::sync::OnceLock;
use std::time::Duration;

use opentelemetry::metrics::{Counter, Histogram, MeterProvider};
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::runtime;
use opentelemetry_sdk::Resource;
use rex_config::ObservabilityConfig;

use crate::economics_record::StreamEconomicsRecord;

static DEGRADED_LOG: OnceLock<()> = OnceLock::new();

/// Optional terminal-time fields not stored on `StreamEconomicsRecord`.
#[derive(Debug, Clone, Default)]
pub struct TerminalOtlpContext {
    pub ttft_ms: Option<u64>,
    pub approval_outcome: Option<String>,
    pub error_type: Option<String>,
    pub broker_inference_outcome: Option<String>,
    pub load_duration_ms: Option<u64>,
}

pub struct OtlpMetrics {
    _provider: SdkMeterProvider,
    stream_requests: Counter<u64>,
    stream_duration_ms: Histogram<u64>,
    cache_decisions: Counter<u64>,
    prompt_tokens: Histogram<u64>,
    context_tokens: Histogram<u64>,
    operation_duration: Histogram<f64>,
    token_usage: Histogram<u64>,
    time_to_first_chunk: Histogram<u64>,
    retrieval_duration: Histogram<u64>,
    compression_ratio: Histogram<f64>,
    load_duration: Histogram<u64>,
    approval_decisions: Counter<u64>,
    broker_inference: Counter<u64>,
}

impl OtlpMetrics {
    pub fn from_config(obs: &ObservabilityConfig) -> Option<Self> {
        let endpoint = obs.otlp.endpoint.trim();
        if endpoint.is_empty() {
            log_export_degraded("missing_endpoint");
            return None;
        }
        let protocol = obs.otlp.protocol.trim().to_ascii_lowercase();
        let exporter = match protocol.as_str() {
            "http/protobuf" | "http-protobuf" => opentelemetry_otlp::MetricExporter::builder()
                .with_http()
                .with_endpoint(endpoint)
                .build(),
            _ => opentelemetry_otlp::MetricExporter::builder()
                .with_tonic()
                .with_endpoint(endpoint)
                .build(),
        };
        let exporter = match exporter {
            Ok(exporter) => exporter,
            Err(err) => {
                log_export_degraded(&format!("exporter_build:{err}"));
                return None;
            }
        };
        let reader = PeriodicReader::builder(exporter, runtime::Tokio)
            .with_interval(Duration::from_secs(5))
            .build();
        let service_name = if obs.service_name.trim().is_empty() {
            rex_config::DEFAULT_OBS_SERVICE_NAME
        } else {
            obs.service_name.trim()
        };
        let resource = Resource::new([KeyValue::new("service.name", service_name.to_string())]);
        let provider = SdkMeterProvider::builder()
            .with_reader(reader)
            .with_resource(resource)
            .build();
        let meter = provider.meter("rex-daemon");
        Some(Self {
            stream_requests: meter
                .u64_counter("rex.stream.requests")
                .with_description("Stream terminal outcomes")
                .build(),
            stream_duration_ms: meter
                .u64_histogram("rex.stream.duration_ms")
                .with_description("Stream elapsed milliseconds")
                .with_unit("ms")
                .build(),
            cache_decisions: meter
                .u64_counter("rex.cache.decisions")
                .with_description("Cache policy decisions")
                .build(),
            prompt_tokens: meter
                .u64_histogram("rex.context.prompt_tokens")
                .with_description("Estimated prompt tokens")
                .build(),
            context_tokens: meter
                .u64_histogram("rex.context.selected_tokens")
                .with_description("Selected context tokens")
                .build(),
            operation_duration: meter
                .f64_histogram("gen_ai.client.operation.duration")
                .with_description("Client operation duration")
                .with_unit("ms")
                .build(),
            token_usage: meter
                .u64_histogram("gen_ai.client.token.usage")
                .with_description("Token usage by type")
                .build(),
            time_to_first_chunk: meter
                .u64_histogram("gen_ai.client.operation.time_to_first_chunk")
                .with_description("Time to first streamed chunk")
                .with_unit("ms")
                .build(),
            retrieval_duration: meter
                .u64_histogram("rex.pipeline.retrieval.duration")
                .with_description("Retrieval phase duration")
                .with_unit("ms")
                .build(),
            compression_ratio: meter
                .f64_histogram("rex.pipeline.compression.ratio")
                .with_description("Context compression ratio")
                .build(),
            load_duration: meter
                .u64_histogram("rex.local.hardware.load_duration")
                .with_description("Local model load duration")
                .with_unit("ms")
                .build(),
            approval_decisions: meter
                .u64_counter("rex.approval.decisions")
                .with_description("Agent approval gate outcomes")
                .build(),
            broker_inference: meter
                .u64_counter("rex.broker.inference")
                .with_description("Broker inference RPC outcomes")
                .build(),
            _provider: provider,
        })
    }

    pub fn record_stream(&self, record: &StreamEconomicsRecord, ctx: &TerminalOtlpContext) {
        let terminal = KeyValue::new("terminal", record.terminal.clone());
        let runtime = KeyValue::new("inference_runtime", record.inference_runtime.clone());
        let route = KeyValue::new("route", record.route.clone());
        let mode = KeyValue::new("mode", record.mode.clone());
        let cache = KeyValue::new("decision", record.cache_decision.clone());
        let model = KeyValue::new("model_id", record.model.clone());
        let error_type = KeyValue::new("error.type", ctx.error_type.clone().unwrap_or_default());

        self.stream_requests
            .add(1, &[terminal.clone(), runtime.clone(), route.clone()]);
        self.stream_duration_ms.record(
            record.elapsed_ms,
            &[terminal.clone(), runtime.clone(), route.clone()],
        );
        self.cache_decisions.add(1, &[cache]);
        self.prompt_tokens
            .record(record.prompt_tokens, &[mode.clone(), route.clone()]);
        self.context_tokens.record(
            record.context_tokens,
            &[mode.clone(), KeyValue::new("route", record.route.clone())],
        );
        self.operation_duration.record(
            record.elapsed_ms as f64,
            &[model.clone(), route.clone(), error_type.clone()],
        );

        self.token_usage.record(
            record.prompt_tokens,
            &[
                KeyValue::new("gen_ai.token.type", "input"),
                model.clone(),
                route.clone(),
            ],
        );
        let completion_estimate = record.chunks_sent.saturating_mul(32);
        if completion_estimate > 0 {
            self.token_usage.record(
                completion_estimate,
                &[
                    KeyValue::new("gen_ai.token.type", "output"),
                    model.clone(),
                    route.clone(),
                ],
            );
        }

        if let Some(ttft) = ctx.ttft_ms {
            self.time_to_first_chunk
                .record(ttft, &[model.clone(), route.clone()]);
        }

        let retrieval_status = KeyValue::new("retrieval_status", record.retrieval.clone());
        let retrieval_ms = if record.retrieval == "skipped" {
            0
        } else {
            record.context_candidates.saturating_mul(2)
        };
        self.retrieval_duration
            .record(retrieval_ms, &[retrieval_status]);

        let ratio = if record.prompt_tokens > 0 {
            record.context_tokens as f64 / record.prompt_tokens as f64
        } else {
            0.0
        };
        self.compression_ratio.record(
            ratio,
            &[KeyValue::new(
                "compression_strategy",
                record.compression_strategy.clone(),
            )],
        );

        if let Some(load_ms) = ctx.load_duration_ms {
            self.load_duration.record(
                load_ms,
                &[model.clone(), KeyValue::new("quant", "unknown".to_string())],
            );
        }

        if let Some(outcome) = ctx.approval_outcome.as_ref() {
            self.approval_decisions
                .add(1, &[KeyValue::new("outcome", outcome.clone())]);
        }

        if let Some(outcome) = ctx.broker_inference_outcome.as_ref() {
            self.broker_inference
                .add(1, &[KeyValue::new("outcome", outcome.clone())]);
        }

        if let Some(cached) = record.cached_tokens {
            self.token_usage.record(
                cached,
                &[
                    KeyValue::new("gen_ai.token.type", "cached_input"),
                    model,
                    route,
                ],
            );
        }
    }
}

pub fn log_export_degraded(reason: &str) {
    let reason = reason.to_string();
    let _ = DEGRADED_LOG.get_or_init(|| {
        eprintln!("obs.export=degraded reason={reason}");
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use rex_config::ObservabilityConfig;

    #[test]
    fn missing_endpoint_logs_degraded_without_panic() {
        let obs = ObservabilityConfig {
            enabled: Some(true),
            ..Default::default()
        };
        assert!(OtlpMetrics::from_config(&obs).is_none());
    }

    #[test]
    fn terminal_context_defaults_are_empty() {
        let ctx = TerminalOtlpContext::default();
        assert!(ctx.ttft_ms.is_none());
        assert!(ctx.approval_outcome.is_none());
    }
}

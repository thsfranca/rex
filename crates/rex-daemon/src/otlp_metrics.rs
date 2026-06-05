use std::sync::OnceLock;
use std::time::Duration;

use opentelemetry::metrics::{Counter, Histogram, MeterProvider};
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::runtime;
use opentelemetry_sdk::Resource;
use rex_config::ObservabilityConfig;
use rex_obs_store::StreamEconomicsRecord;

static DEGRADED_LOG: OnceLock<()> = OnceLock::new();

pub struct OtlpMetrics {
    _provider: SdkMeterProvider,
    stream_requests: Counter<u64>,
    stream_duration_ms: Histogram<u64>,
    cache_decisions: Counter<u64>,
    prompt_tokens: Histogram<u64>,
    context_tokens: Histogram<u64>,
    operation_duration: Histogram<f64>,
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
            _provider: provider,
        })
    }

    pub fn record_stream(&self, record: &StreamEconomicsRecord) {
        let terminal = KeyValue::new("terminal", record.terminal.clone());
        let runtime = KeyValue::new("inference_runtime", record.inference_runtime.clone());
        let route = KeyValue::new("route", record.route.clone());
        let mode = KeyValue::new("mode", record.mode.clone());
        let cache = KeyValue::new("decision", record.cache_decision.clone());

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
            &[
                KeyValue::new("model_id", record.model.clone()),
                KeyValue::new("route", record.route.clone()),
            ],
        );
    }
}

fn log_export_degraded(reason: &str) {
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
}

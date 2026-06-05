use serde::{Deserialize, Serialize};

use crate::query::QueriedStream;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct InstrumentCatalogEntry {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub unit: String,
    pub description: String,
    #[serde(rename = "labelKeys")]
    pub label_keys: Vec<String>,
}

pub fn instrument_catalog() -> Vec<InstrumentCatalogEntry> {
    vec![
        InstrumentCatalogEntry {
            name: "rex.stream.requests".to_string(),
            kind: "counter".to_string(),
            unit: "1".to_string(),
            description: "Stream terminal outcomes".to_string(),
            label_keys: vec![
                "terminal".into(),
                "inference_runtime".into(),
                "route".into(),
            ],
        },
        InstrumentCatalogEntry {
            name: "rex.stream.duration_ms".to_string(),
            kind: "histogram".to_string(),
            unit: "ms".to_string(),
            description: "Stream elapsed milliseconds".to_string(),
            label_keys: vec![
                "terminal".into(),
                "inference_runtime".into(),
                "route".into(),
            ],
        },
        InstrumentCatalogEntry {
            name: "rex.cache.decisions".to_string(),
            kind: "counter".to_string(),
            unit: "1".to_string(),
            description: "Cache policy decisions".to_string(),
            label_keys: vec!["decision".into()],
        },
        InstrumentCatalogEntry {
            name: "rex.context.prompt_tokens".to_string(),
            kind: "histogram".to_string(),
            unit: "1".to_string(),
            description: "Estimated prompt tokens".to_string(),
            label_keys: vec!["mode".into(), "route".into()],
        },
        InstrumentCatalogEntry {
            name: "rex.context.selected_tokens".to_string(),
            kind: "histogram".to_string(),
            unit: "1".to_string(),
            description: "Selected context tokens".to_string(),
            label_keys: vec!["mode".into(), "route".into()],
        },
        InstrumentCatalogEntry {
            name: "gen_ai.client.operation.duration".to_string(),
            kind: "histogram".to_string(),
            unit: "ms".to_string(),
            description: "Client operation duration".to_string(),
            label_keys: vec!["model_id".into(), "route".into()],
        },
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricsQueryRequest {
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
    pub instruments: Vec<String>,
    #[serde(default)]
    pub labels: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MetricsQueryResponse {
    #[serde(rename = "resourceMetrics")]
    pub resource_metrics: Vec<ResourceMetrics>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ResourceMetrics {
    pub resource: OtelResource,
    #[serde(rename = "scopeMetrics")]
    pub scope_metrics: Vec<ScopeMetrics>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OtelResource {
    pub attributes: Vec<OtelAttribute>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ScopeMetrics {
    pub scope: OtelScope,
    pub metrics: Vec<OtelMetric>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OtelScope {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OtelMetric {
    pub name: String,
    pub unit: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sum: Option<OtelSum>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub histogram: Option<OtelHistogram>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OtelSum {
    #[serde(rename = "aggregationTemporality")]
    pub aggregation_temporality: u8,
    #[serde(rename = "isMonotonic")]
    pub is_monotonic: bool,
    #[serde(rename = "dataPoints")]
    pub data_points: Vec<OtelNumberDataPoint>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OtelHistogram {
    #[serde(rename = "aggregationTemporality")]
    pub aggregation_temporality: u8,
    #[serde(rename = "dataPoints")]
    pub data_points: Vec<OtelHistogramDataPoint>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OtelNumberDataPoint {
    #[serde(rename = "timeUnixNano")]
    pub time_unix_nano: String,
    #[serde(rename = "asInt", skip_serializing_if = "Option::is_none")]
    pub as_int: Option<String>,
    #[serde(rename = "asDouble", skip_serializing_if = "Option::is_none")]
    pub as_double: Option<f64>,
    pub attributes: Vec<OtelAttribute>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OtelHistogramDataPoint {
    #[serde(rename = "timeUnixNano")]
    pub time_unix_nano: String,
    pub count: String,
    pub sum: f64,
    #[serde(rename = "bucketCounts")]
    pub bucket_counts: Vec<String>,
    #[serde(rename = "explicitBounds")]
    pub explicit_bounds: Vec<f64>,
    pub attributes: Vec<OtelAttribute>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct OtelAttribute {
    pub key: String,
    pub value: OtelAttributeValue,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct OtelAttributeValue {
    #[serde(rename = "stringValue")]
    pub string_value: String,
}

pub fn project_metrics(
    service_name: &str,
    streams: &[QueriedStream],
    request: &MetricsQueryRequest,
) -> MetricsQueryResponse {
    let catalog = instrument_catalog();
    let wanted: std::collections::BTreeSet<&str> = if request.instruments.is_empty() {
        catalog.iter().map(|e| e.name.as_str()).collect()
    } else {
        request.instruments.iter().map(String::as_str).collect()
    };

    let mut metrics = Vec::new();
    for entry in &catalog {
        if !wanted.contains(entry.name.as_str()) {
            continue;
        }
        if let Some(metric) = build_metric(entry.name.as_str(), streams, request) {
            metrics.push(metric);
        }
    }

    MetricsQueryResponse {
        resource_metrics: vec![ResourceMetrics {
            resource: OtelResource {
                attributes: vec![attr("service.name", service_name)],
            },
            scope_metrics: vec![ScopeMetrics {
                scope: OtelScope {
                    name: "rex-obs-read-api".to_string(),
                },
                metrics,
            }],
        }],
    }
}

fn build_metric(
    name: &str,
    streams: &[QueriedStream],
    request: &MetricsQueryRequest,
) -> Option<OtelMetric> {
    let entry = instrument_catalog().into_iter().find(|e| e.name == name)?;
    let filtered: Vec<&QueriedStream> =
        streams.iter().filter(|s| label_match(s, request)).collect();
    if filtered.is_empty() {
        return None;
    }

    match entry.kind.as_str() {
        "counter" => Some(OtelMetric {
            name: entry.name,
            unit: entry.unit,
            description: entry.description,
            sum: Some(OtelSum {
                aggregation_temporality: 2,
                is_monotonic: true,
                data_points: filtered.iter().map(|s| counter_point(name, s)).collect(),
            }),
            histogram: None,
        }),
        "histogram" => Some(OtelMetric {
            name: entry.name.clone(),
            unit: entry.unit,
            description: entry.description,
            sum: None,
            histogram: Some(OtelHistogram {
                aggregation_temporality: 2,
                data_points: filtered.iter().map(|s| histogram_point(name, s)).collect(),
            }),
        }),
        _ => None,
    }
}

fn label_match(stream: &QueriedStream, request: &MetricsQueryRequest) -> bool {
    for (key, want) in &request.labels {
        let got = match key.as_str() {
            "terminal" => &stream.record.terminal,
            "route" => &stream.record.route,
            "mode" => &stream.record.mode,
            "decision" => &stream.record.cache_decision,
            "inference_runtime" => &stream.record.inference_runtime,
            "model_id" => &stream.record.model,
            _ => return false,
        };
        if got != want {
            return false;
        }
    }
    true
}

fn counter_point(name: &str, stream: &QueriedStream) -> OtelNumberDataPoint {
    let attrs = match name {
        "rex.stream.requests" => vec![
            attr("terminal", &stream.record.terminal),
            attr("inference_runtime", &stream.record.inference_runtime),
            attr("route", &stream.record.route),
        ],
        "rex.cache.decisions" => vec![attr("decision", &stream.record.cache_decision)],
        _ => Vec::new(),
    };
    OtelNumberDataPoint {
        time_unix_nano: ms_to_nano(stream.created_at_ms),
        as_int: Some("1".to_string()),
        as_double: None,
        attributes: attrs,
    }
}

fn histogram_point(name: &str, stream: &QueriedStream) -> OtelHistogramDataPoint {
    let (value, attrs) = match name {
        "rex.stream.duration_ms" => (
            stream.record.elapsed_ms as f64,
            vec![
                attr("terminal", &stream.record.terminal),
                attr("inference_runtime", &stream.record.inference_runtime),
                attr("route", &stream.record.route),
            ],
        ),
        "rex.context.prompt_tokens" => (
            stream.record.prompt_tokens as f64,
            vec![
                attr("mode", &stream.record.mode),
                attr("route", &stream.record.route),
            ],
        ),
        "rex.context.selected_tokens" => (
            stream.record.context_tokens as f64,
            vec![
                attr("mode", &stream.record.mode),
                attr("route", &stream.record.route),
            ],
        ),
        "gen_ai.client.operation.duration" => (
            stream.record.elapsed_ms as f64,
            vec![
                attr("model_id", &stream.record.model),
                attr("route", &stream.record.route),
            ],
        ),
        _ => (0.0, Vec::new()),
    };
    OtelHistogramDataPoint {
        time_unix_nano: ms_to_nano(stream.created_at_ms),
        count: "1".to_string(),
        sum: value,
        bucket_counts: vec!["0".into(), "1".into(), "0".into()],
        explicit_bounds: vec![0.0, value.max(1.0)],
        attributes: attrs,
    }
}

fn attr(key: &str, value: &str) -> OtelAttribute {
    OtelAttribute {
        key: key.to_string(),
        value: OtelAttributeValue {
            string_value: value.to_string(),
        },
    }
}

fn ms_to_nano(ms: i64) -> String {
    (ms.saturating_mul(1_000_000)).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::QueriedStream;
    use crate::StreamEconomicsRecord;

    fn one_stream() -> QueriedStream {
        QueriedStream {
            record: StreamEconomicsRecord {
                snapshot_id: "snap".into(),
                request_id: 1,
                trace_id: "t".into(),
                turn_id: "".into(),
                terminal: "done".into(),
                route: "sidecar+mock".into(),
                cache_decision: "miss_stored".into(),
                decision_id: "dec-1".into(),
                inference_runtime: "mock".into(),
                mode: "ask".into(),
                model: "gpt-4o-mini".into(),
                elapsed_ms: 100,
                chunks_sent: 1,
                prompt_tokens: 10,
                context_tokens: 5,
                context_candidates: 1,
                context_selected: 1,
                context_truncated: false,
                retrieval: "skipped".into(),
                compression_strategy: "none".into(),
                cached_tokens: None,
                prefix_hash: None,
                parse_retries: None,
            },
            created_at_ms: 1_700_000_000_000,
        }
    }

    #[test]
    fn catalog_lists_core_instruments() {
        let catalog = instrument_catalog();
        let names: Vec<_> = catalog.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"rex.stream.requests"));
        assert!(names.contains(&"gen_ai.client.operation.duration"));
    }

    #[test]
    fn project_includes_stream_counter() {
        let resp = project_metrics(
            "rex-daemon",
            &[one_stream()],
            &MetricsQueryRequest {
                start_ms: None,
                end_ms: None,
                instruments: vec!["rex.stream.requests".into()],
                labels: Default::default(),
            },
        );
        let metric = &resp.resource_metrics[0].scope_metrics[0].metrics[0];
        assert_eq!(metric.name, "rex.stream.requests");
        assert!(metric.sum.is_some());
    }
}

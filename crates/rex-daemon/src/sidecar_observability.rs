use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rex_config::observability_enabled;
use rex_obs_store::{
    SharedObsStore, SidecarMetricDef, SpanRecord, StreamQueryFilter, TailTelemetryEvent,
};
use rex_proto::rex::observability::v1::sidecar_observability_service_server::SidecarObservabilityService;
use rex_proto::rex::observability::v1::{
    AppendSpanRequest, AppendSpanResponse, EconomicsSummary, GetEconomicsSnapshotRequest,
    GetEconomicsSnapshotResponse, RecordMetricRequest, RecordMetricResponse, RegisterMetricRequest,
    RegisterMetricResponse, ReportResourceStatsRequest, ReportResourceStatsResponse,
};
use tonic::{Request, Response, Status};

use crate::observability::ObservabilityRuntime;
use crate::settings;

#[derive(Default)]
struct CustomMetricCatalog {
    defs: HashMap<String, SidecarMetricDef>,
}

pub struct SidecarObservabilityHandler {
    observability: Option<Arc<ObservabilityRuntime>>,
    catalog: Mutex<CustomMetricCatalog>,
}

impl SidecarObservabilityHandler {
    pub fn new(observability: Option<Arc<ObservabilityRuntime>>) -> Self {
        Self {
            observability,
            catalog: Mutex::new(CustomMetricCatalog::default()),
        }
    }

    fn store(&self) -> Result<&SharedObsStore, Status> {
        self.observability
            .as_ref()
            .map(|obs| obs.store())
            .ok_or_else(|| Status::failed_precondition("observability disabled"))
    }

    fn custom_metrics_enabled(&self) -> bool {
        settings::get()
            .effective
            .observability
            .custom_sidecar_metrics
    }
}

#[tonic::async_trait]
impl SidecarObservabilityService for SidecarObservabilityHandler {
    async fn register_metric(
        &self,
        request: Request<RegisterMetricRequest>,
    ) -> Result<Response<RegisterMetricResponse>, Status> {
        if !self.custom_metrics_enabled() {
            return Ok(Response::new(RegisterMetricResponse {
                ok: false,
                instrument_name: String::new(),
                error: "custom_sidecar_metrics disabled".to_string(),
            }));
        }
        let inner = request.into_inner();
        if inner.name.trim().is_empty() {
            return Ok(Response::new(RegisterMetricResponse {
                ok: false,
                instrument_name: String::new(),
                error: "metric name required".to_string(),
            }));
        }
        let def = SidecarMetricDef {
            name: inner.name.clone(),
            kind: if inner.kind.is_empty() {
                "counter".to_string()
            } else {
                inner.kind
            },
            unit: inner.unit,
            description: inner.description,
            label_keys: inner.label_keys,
        };
        {
            let mut catalog = self
                .catalog
                .lock()
                .map_err(|_| Status::internal("sidecar metric catalog lock poisoned"))?;
            catalog.defs.insert(def.name.clone(), def.clone());
        }
        if observability_enabled(&settings::get().effective.observability) {
            if let Ok(store) = self.store() {
                let _ = store.register_sidecar_metric(&def);
            }
        }
        Ok(Response::new(RegisterMetricResponse {
            ok: true,
            instrument_name: format!("rex.sidecar.custom.{}", inner.name),
            error: String::new(),
        }))
    }

    async fn record_metric(
        &self,
        request: Request<RecordMetricRequest>,
    ) -> Result<Response<RecordMetricResponse>, Status> {
        if !self.custom_metrics_enabled() {
            return Ok(Response::new(RecordMetricResponse {
                ok: false,
                error: "custom_sidecar_metrics disabled".to_string(),
            }));
        }
        let inner = request.into_inner();
        let registered = {
            let catalog = self
                .catalog
                .lock()
                .map_err(|_| Status::internal("sidecar metric catalog lock poisoned"))?;
            catalog.defs.contains_key(&inner.name)
        };
        if !registered {
            return Ok(Response::new(RecordMetricResponse {
                ok: false,
                error: format!("metric `{}` not registered", inner.name),
            }));
        }
        let timestamp_ms = if inner.timestamp_ms > 0 {
            inner.timestamp_ms
        } else {
            chrono_now_ms()
        };
        if let Some(obs) = self.observability.as_ref() {
            if let Ok(tail) = obs.store().tail() {
                let instrument = format!("rex.sidecar.custom.{}", inner.name);
                let attrs: Vec<serde_json::Value> = inner
                    .labels
                    .iter()
                    .map(|(k, v)| {
                        serde_json::json!({
                            "key": k,
                            "value": { "stringValue": v }
                        })
                    })
                    .collect();
                let data_point = serde_json::json!({
                    "timeUnixNano": (timestamp_ms.saturating_mul(1_000_000)).to_string(),
                    "asDouble": inner.value,
                    "attributes": attrs,
                });
                tail.publish(TailTelemetryEvent {
                    timestamp_ms,
                    instrument,
                    data_point,
                });
            }
        }
        Ok(Response::new(RecordMetricResponse {
            ok: true,
            error: String::new(),
        }))
    }

    async fn get_economics_snapshot(
        &self,
        request: Request<GetEconomicsSnapshotRequest>,
    ) -> Result<Response<GetEconomicsSnapshotResponse>, Status> {
        let limit = request.into_inner().limit.clamp(1, 100) as usize;
        let store = self.store()?;
        let rows = store
            .query_streams(&StreamQueryFilter::default())
            .map_err(|err| Status::internal(format!("store query failed: {err}")))?;
        let streams = rows
            .into_iter()
            .rev()
            .take(limit)
            .map(|row| EconomicsSummary {
                request_id: row.record.request_id,
                trace_id: row.record.trace_id,
                terminal: row.record.terminal,
                created_at_ms: row.created_at_ms,
                route: row.record.route,
            })
            .collect();
        Ok(Response::new(GetEconomicsSnapshotResponse { streams }))
    }

    async fn report_resource_stats(
        &self,
        request: Request<ReportResourceStatsRequest>,
    ) -> Result<Response<ReportResourceStatsResponse>, Status> {
        let inner = request.into_inner();
        println!(
            "sidecar.resource_stats cpu_percent={} memory_bytes={}",
            inner.cpu_percent, inner.memory_bytes
        );
        Ok(Response::new(ReportResourceStatsResponse { ok: true }))
    }

    async fn append_span(
        &self,
        request: Request<AppendSpanRequest>,
    ) -> Result<Response<AppendSpanResponse>, Status> {
        let inner = request.into_inner();
        if inner.trace_id.trim().is_empty() || inner.span_name.trim().is_empty() {
            return Ok(Response::new(AppendSpanResponse {
                ok: false,
                error: "trace_id and span_name required".to_string(),
            }));
        }
        let attributes_json = serde_json::to_string(&inner.attributes)
            .map_err(|err| Status::invalid_argument(format!("attributes invalid: {err}")))?;
        let span = SpanRecord {
            trace_id: inner.trace_id,
            turn_id: inner.turn_id,
            span_name: inner.span_name,
            parent_span_id: if inner.parent_span_id.is_empty() {
                None
            } else {
                Some(inner.parent_span_id)
            },
            start_ms: inner.start_ms,
            end_ms: if inner.end_ms > 0 {
                Some(inner.end_ms)
            } else {
                None
            },
            attributes_json,
        };
        self.store()?
            .append_span(&span)
            .map_err(|err| Status::internal(format!("append_span failed: {err}")))?;
        Ok(Response::new(AppendSpanResponse {
            ok: true,
            error: String::new(),
        }))
    }
}

fn chrono_now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

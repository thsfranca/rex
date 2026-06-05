use std::sync::Arc;

use rex_config::{
    economics_snapshot_id, economics_snapshot_json, observability_enabled, resolve_store_path,
    LoadedConfig,
};
use rex_obs_store::{SharedObsStore, StreamEconomicsRecord};

use crate::plugins::PipelineMetrics;

#[derive(Clone)]
pub struct StreamEconomicsDraft {
    pub snapshot_id: String,
    pub request_id: u64,
    pub trace_id: String,
    pub turn_id: String,
    pub route: String,
    pub cache_decision: String,
    pub decision_id: String,
    pub inference_runtime: String,
    pub mode: String,
    pub model: String,
    pub metrics: PipelineMetrics,
}

impl StreamEconomicsDraft {
    pub fn into_record(
        self,
        terminal: &str,
        elapsed_ms: u64,
        chunks_sent: u64,
    ) -> StreamEconomicsRecord {
        StreamEconomicsRecord {
            snapshot_id: self.snapshot_id,
            request_id: self.request_id,
            trace_id: self.trace_id,
            turn_id: self.turn_id,
            terminal: terminal.to_string(),
            route: self.route,
            cache_decision: self.cache_decision,
            decision_id: self.decision_id,
            inference_runtime: self.inference_runtime,
            mode: self.mode,
            model: self.model,
            elapsed_ms,
            chunks_sent,
            prompt_tokens: self.metrics.prompt_tokens as u64,
            context_tokens: self.metrics.selected_context_tokens as u64,
            context_candidates: self.metrics.context_candidates as u64,
            context_selected: self.metrics.context_selected as u64,
            context_truncated: self.metrics.context_truncated,
            retrieval: self.metrics.retrieval.as_str().to_string(),
            compression_strategy: self.metrics.compression_strategy.to_string(),
            cached_tokens: None,
            prefix_hash: None,
            parse_retries: None,
        }
    }
}

#[derive(Clone)]
pub struct ObservabilityRuntime {
    store: SharedObsStore,
    snapshot_id: String,
}

impl ObservabilityRuntime {
    pub fn from_loaded(
        loaded: &LoadedConfig,
    ) -> Result<Option<Self>, rex_obs_store::ObsStoreError> {
        if !observability_enabled(&loaded.effective.observability) {
            return Ok(None);
        }
        let path = resolve_store_path(&loaded.rex_root, &loaded.effective.observability.store);
        let store = SharedObsStore::open(path)?;
        let snapshot_id = economics_snapshot_id(&loaded.effective);
        let payload = economics_snapshot_json(&loaded.effective).to_string();
        store.upsert_config_snapshot(&snapshot_id, &payload)?;
        Ok(Some(Self { store, snapshot_id }))
    }

    pub fn snapshot_id(&self) -> &str {
        &self.snapshot_id
    }

    pub fn record_terminal_async(
        &self,
        draft: StreamEconomicsDraft,
        terminal: &str,
        elapsed_ms: u64,
        chunks_sent: u64,
    ) {
        let store = self.store.clone();
        let record = draft.into_record(terminal, elapsed_ms, chunks_sent);
        tokio::task::spawn_blocking(move || {
            if let Err(err) = store.append_stream(&record) {
                eprintln!("obs.store=degraded reason=append_failed error={err}");
            }
        });
    }
}

pub fn observability_from_settings() -> Option<Arc<ObservabilityRuntime>> {
    let loaded = crate::settings::get();
    if !observability_enabled(&loaded.effective.observability) {
        return None;
    }
    match ObservabilityRuntime::from_loaded(&loaded) {
        Ok(Some(runtime)) => Some(Arc::new(runtime)),
        Ok(None) => None,
        Err(err) => {
            eprintln!("obs.store=degraded reason=open_failed error={err}");
            None
        }
    }
}

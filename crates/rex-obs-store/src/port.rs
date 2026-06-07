use std::path::Path;

use crate::error::ObsStoreError;
use crate::query::ObsQuery;
use crate::record::StreamEconomicsRecord;

/// Logical write/read contract shared by SQLite and CHCE engines.
pub trait StorePort: ObsQuery {
    fn path(&self) -> &Path;

    fn upsert_config_snapshot(
        &self,
        snapshot_id: &str,
        payload_json: &str,
    ) -> Result<(), ObsStoreError>;

    fn append_stream(&self, record: &StreamEconomicsRecord) -> Result<(), ObsStoreError>;

    fn stream_count(&self) -> Result<u64, ObsStoreError>;
}

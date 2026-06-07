mod dict;
mod page;
mod ring;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::error::ObsStoreError;
use crate::port::StorePort;
use crate::query::{ObsQuery, QueriedStream, StreamQueryFilter};
use crate::record::StreamEconomicsRecord;

use dict::DictionaryManager;
use page::{ColumnarCodec, MmapPaginator, SealedPage};
use ring::{LiveRingBuffer, RingPushError, StreamAppendEvent};

/// CHCE mmap engine — macOS opt-in when `observability.store.engine=mmap`.
pub struct ChceEngine {
    obs_dir: PathBuf,
    ring: LiveRingBuffer,
    paginator: Arc<Mutex<MmapPaginator>>,
    dict: Arc<Mutex<DictionaryManager>>,
    configs: Arc<Mutex<ConfigStore>>,
    writer: Option<WriterHandle>,
}

struct WriterHandle {
    shutdown: Arc<AtomicBool>,
    join: JoinHandle<()>,
}

struct ConfigStore {
    path: PathBuf,
    snapshots: HashMap<String, ConfigEntry>,
}

#[derive(Debug, Clone)]
struct ConfigEntry {
    payload_json: String,
    created_at_ms: u64,
}

impl std::fmt::Debug for ChceEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChceEngine")
            .field("obs_dir", &self.obs_dir)
            .finish_non_exhaustive()
    }
}

impl ChceEngine {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ObsStoreError> {
        let path = path.as_ref().to_path_buf();
        let obs_dir = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let rexobs_path = obs_dir.join("store.rexobs");
        let dict_path = obs_dir.join("store.dict");
        let config_path = obs_dir.join("store.config");

        let dict = Arc::new(Mutex::new(DictionaryManager::open(dict_path)?));
        let configs = Arc::new(Mutex::new(ConfigStore::open(config_path)?));
        let paginator = Arc::new(Mutex::new(MmapPaginator::open(rexobs_path)?));

        let mut ring = LiveRingBuffer::new(4096);
        let shutdown = Arc::new(AtomicBool::new(false));
        let receiver = ring.take_receiver();
        let writer_shutdown = Arc::clone(&shutdown);
        let writer_paginator = Arc::clone(&paginator);
        let writer_dict = Arc::clone(&dict);

        let join = thread::Builder::new()
            .name("chce-writer".into())
            .spawn(move || {
                writer_loop(receiver, writer_paginator, writer_dict, writer_shutdown);
            })?;

        Ok(Self {
            obs_dir,
            ring,
            paginator,
            dict,
            configs,
            writer: Some(WriterHandle { shutdown, join }),
        })
    }

    pub fn obs_dir(&self) -> &Path {
        &self.obs_dir
    }

    /// Block until the background writer drains pending append events (tests).
    pub fn drain_for_test(&self) -> Result<(), ObsStoreError> {
        for _ in 0..200 {
            if self.ring.is_empty() {
                // Writer flushes buffered batches on recv timeout (~50ms).
                thread::sleep(Duration::from_millis(75));
                return Ok(());
            }
            thread::sleep(Duration::from_millis(5));
        }
        Err(ObsStoreError::Io(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "CHCE writer did not drain pending events",
        )))
    }

    fn query_committed_streams(&self) -> Result<Vec<QueriedStream>, ObsStoreError> {
        let paginator = self.paginator.lock().map_err(lock_err)?;
        let dict = self.dict.lock().map_err(lock_err)?;
        let mut rows = Vec::new();

        for page in paginator.iter_valid_pages()? {
            let batch = ColumnarCodec::decode_page(&page, &dict)?;
            rows.extend(batch.into_queried_streams(&dict));
        }
        Ok(rows)
    }
}

impl Drop for ChceEngine {
    fn drop(&mut self) {
        if let Some(writer) = self.writer.take() {
            writer.shutdown.store(true, Ordering::Release);
            let _ = writer.join.join();
        }
    }
}

impl StorePort for ChceEngine {
    fn path(&self) -> &Path {
        &self.obs_dir
    }

    fn upsert_config_snapshot(
        &self,
        snapshot_id: &str,
        payload_json: &str,
    ) -> Result<(), ObsStoreError> {
        let mut configs = self.configs.lock().map_err(lock_err)?;
        configs.upsert(snapshot_id, payload_json, now_ms())?;
        Ok(())
    }

    fn append_stream(&self, record: &StreamEconomicsRecord) -> Result<(), ObsStoreError> {
        {
            let configs = self.configs.lock().map_err(lock_err)?;
            if !configs.contains(&record.snapshot_id) {
                return Err(ObsStoreError::UnknownSnapshot(record.snapshot_id.clone()));
            }
        }

        let event = StreamAppendEvent {
            record: record.clone(),
            created_at_ms: now_ms(),
        };
        self.ring.push(event).map_err(|err| match err {
            RingPushError::Full => ObsStoreError::Io(std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "CHCE ring buffer full",
            )),
        })
    }

    fn stream_count(&self) -> Result<u64, ObsStoreError> {
        let paginator = self.paginator.lock().map_err(lock_err)?;
        paginator.total_record_count()
    }
}

impl ObsQuery for ChceEngine {
    fn query_streams(
        &self,
        filter: &StreamQueryFilter,
    ) -> Result<Vec<QueriedStream>, ObsStoreError> {
        let mut rows = self.query_committed_streams()?;
        rows.sort_by_key(|row| row.created_at_ms);
        Ok(apply_stream_filter(rows, filter))
    }
}

fn writer_loop(
    ring: ring::RingReceiver,
    paginator: Arc<Mutex<MmapPaginator>>,
    dict: Arc<Mutex<DictionaryManager>>,
    shutdown: Arc<AtomicBool>,
) {
    let mut pending: Vec<StreamAppendEvent> = Vec::new();

    loop {
        if shutdown.load(Ordering::Acquire) {
            drain_pending(&mut pending, &paginator, &dict);
            return;
        }

        match ring.recv_timeout(Duration::from_millis(50)) {
            Ok(event) => pending.push(event),
            Err(ring::RingRecvError::Timeout) => {
                if !pending.is_empty() {
                    flush_batch(&mut pending, &paginator, &dict);
                }
            }
            Err(ring::RingRecvError::Disconnected) => {
                drain_pending(&mut pending, &paginator, &dict);
                return;
            }
        }

        if pending.len() >= 64 {
            flush_batch(&mut pending, &paginator, &dict);
        }
    }
}

fn flush_batch(
    pending: &mut Vec<StreamAppendEvent>,
    paginator: &Arc<Mutex<MmapPaginator>>,
    dict: &Arc<Mutex<DictionaryManager>>,
) {
    if pending.is_empty() {
        return;
    }
    let batch = std::mem::take(pending);
    if let Err(err) = seal_batch(batch, paginator, dict) {
        eprintln!("chce writer seal failed: {err}");
    }
}

fn drain_pending(
    pending: &mut Vec<StreamAppendEvent>,
    paginator: &Arc<Mutex<MmapPaginator>>,
    dict: &Arc<Mutex<DictionaryManager>>,
) {
    flush_batch(pending, paginator, dict);
}

fn seal_batch(
    events: Vec<StreamAppendEvent>,
    paginator: &Arc<Mutex<MmapPaginator>>,
    dict: &Arc<Mutex<DictionaryManager>>,
) -> Result<(), ObsStoreError> {
    let mut dict_guard = dict.lock().map_err(lock_err)?;
    let encoded = ColumnarCodec::encode_batch(&events, &mut dict_guard)?;
    dict_guard.persist()?;

    let page = SealedPage::seal(encoded)?;
    let mut paginator_guard = paginator.lock().map_err(lock_err)?;
    paginator_guard.append_page(&page)?;
    Ok(())
}

impl ConfigStore {
    fn open(path: PathBuf) -> Result<Self, ObsStoreError> {
        let mut store = Self {
            path,
            snapshots: HashMap::new(),
        };
        store.load()?;
        Ok(store)
    }

    fn contains(&self, snapshot_id: &str) -> bool {
        self.snapshots.contains_key(snapshot_id)
    }

    fn upsert(
        &mut self,
        snapshot_id: &str,
        payload_json: &str,
        created_at_ms: u64,
    ) -> Result<(), ObsStoreError> {
        self.snapshots.insert(
            snapshot_id.to_string(),
            ConfigEntry {
                payload_json: payload_json.to_string(),
                created_at_ms,
            },
        );
        self.persist()
    }

    fn load(&mut self) -> Result<(), ObsStoreError> {
        if !self.path.exists() {
            return Ok(());
        }
        let mut file = File::open(&self.path)?;
        let mut snapshots = HashMap::new();
        loop {
            let id_len = match read_u32(&mut file) {
                Ok(v) => v,
                Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(err) => return Err(err.into()),
            };
            let mut id = vec![0_u8; id_len as usize];
            file.read_exact(&mut id)?;
            let payload_len = read_u32(&mut file)?;
            let mut payload = vec![0_u8; payload_len as usize];
            file.read_exact(&mut payload)?;
            let created_at_ms = read_u64(&mut file)?;
            snapshots.insert(
                String::from_utf8(id).map_err(|_| utf8_err())?,
                ConfigEntry {
                    payload_json: String::from_utf8(payload).map_err(|_| utf8_err())?,
                    created_at_ms,
                },
            );
        }
        self.snapshots = snapshots;
        Ok(())
    }

    fn persist(&self) -> Result<(), ObsStoreError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)?;
        for (id, entry) in &self.snapshots {
            write_u32(&mut file, id.len() as u32)?;
            file.write_all(id.as_bytes())?;
            write_u32(&mut file, entry.payload_json.len() as u32)?;
            file.write_all(entry.payload_json.as_bytes())?;
            write_u64(&mut file, entry.created_at_ms)?;
        }
        file.sync_data()?;
        Ok(())
    }
}

fn apply_stream_filter(rows: Vec<QueriedStream>, filter: &StreamQueryFilter) -> Vec<QueriedStream> {
    rows.into_iter()
        .filter(|row| {
            if let Some(start) = filter.start_ms {
                if row.created_at_ms < start {
                    return false;
                }
            }
            if let Some(end) = filter.end_ms {
                if row.created_at_ms > end {
                    return false;
                }
            }
            if let Some(terminal) = filter.terminal.as_ref() {
                if row.record.terminal != *terminal {
                    return false;
                }
            }
            if let Some(route) = filter.route.as_ref() {
                if row.record.route != *route {
                    return false;
                }
            }
            if let Some(mode) = filter.mode.as_ref() {
                if row.record.mode != *mode {
                    return false;
                }
            }
            if let Some(cache) = filter.cache_decision.as_ref() {
                if row.record.cache_decision != *cache {
                    return false;
                }
            }
            true
        })
        .collect()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn read_u32(file: &mut File) -> Result<u32, std::io::Error> {
    let mut buf = [0_u8; 4];
    file.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u64(file: &mut File) -> Result<u64, std::io::Error> {
    let mut buf = [0_u8; 8];
    file.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn write_u32(file: &mut File, value: u32) -> Result<(), std::io::Error> {
    file.write_all(&value.to_le_bytes())
}

fn write_u64(file: &mut File, value: u64) -> Result<(), std::io::Error> {
    file.write_all(&value.to_le_bytes())
}

fn utf8_err() -> ObsStoreError {
    ObsStoreError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "invalid UTF-8 in CHCE config store",
    ))
}

fn lock_err<T>(_: std::sync::PoisonError<T>) -> ObsStoreError {
    ObsStoreError::Io(std::io::Error::other("CHCE engine lock poisoned"))
}

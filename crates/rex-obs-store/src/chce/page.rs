use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;

use crc32fast::Hasher as Crc32Hasher;
use memmap2::MmapOptions;

use crate::chce::dict::{DictionaryManager, NULL_ORDINAL};
use crate::chce::ring::StreamAppendEvent;
use crate::error::ObsStoreError;
use crate::query::QueriedStream;
use crate::record::StreamEconomicsRecord;

pub const PAGE_SIZE: usize = 16_384;
pub const MAGIC: [u8; 4] = *b"REXO";
pub const FORMAT_VERSION: u16 = 1;
pub const HEADER_SIZE: usize = 64;
pub const ZONE_FOOTER_OFFSET: usize = 0x3FC0;
#[allow(dead_code)]
pub const ZONE_FOOTER_SIZE: usize = 64;
pub const COMMIT_TAIL_SIZE: u64 = 8;
pub const MAX_PAYLOAD_SIZE: usize = ZONE_FOOTER_OFFSET - HEADER_SIZE;

const COL_SNAPSHOT: u8 = 0;
const COL_REQUEST_ID: u8 = 1;
const COL_TRACE_ID: u8 = 2;
const COL_TURN_ID: u8 = 3;
const COL_TERMINAL: u8 = 4;
const COL_ROUTE: u8 = 5;
const COL_CACHE_DECISION: u8 = 6;
const COL_DECISION_ID: u8 = 7;
const COL_INFERENCE_RUNTIME: u8 = 8;
const COL_MODE: u8 = 9;
const COL_MODEL: u8 = 10;
const COL_ELAPSED_MS: u8 = 11;
const COL_CHUNKS_SENT: u8 = 12;
const COL_PROMPT_TOKENS: u8 = 13;
const COL_CONTEXT_TOKENS: u8 = 14;
const COL_CONTEXT_CANDIDATES: u8 = 15;
const COL_CONTEXT_SELECTED: u8 = 16;
const COL_CONTEXT_TRUNCATED: u8 = 17;
const COL_RETRIEVAL: u8 = 18;
const COL_COMPRESSION_STRATEGY: u8 = 19;
const COL_CACHED_TOKENS: u8 = 20;
const COL_PREFIX_HASH: u8 = 21;
const COL_PARSE_RETRIES: u8 = 22;
const COL_CREATED_AT_MS: u8 = 23;

/// Encoded column batch ready for page sealing.
#[derive(Debug, Clone)]
pub struct EncodedBatch {
    pub payload: Vec<u8>,
    pub min_ts: u64,
    pub max_ts: u64,
    pub record_count: u32,
}

/// Decoded rows from one sealed page.
#[derive(Debug, Clone)]
pub struct StreamBatch {
    pub records: Vec<StreamEconomicsRecord>,
    pub created_at_ms: Vec<u64>,
}

impl StreamBatch {
    pub fn into_queried_streams(self, _dict: &DictionaryManager) -> Vec<QueriedStream> {
        self.records
            .into_iter()
            .zip(self.created_at_ms)
            .map(|(record, created_at_ms)| QueriedStream {
                record,
                created_at_ms: created_at_ms as i64,
            })
            .collect()
    }
}

/// Columnar encode/decode for CHCE v1 stream pages.
pub struct ColumnarCodec;

impl ColumnarCodec {
    pub fn encode_batch(
        events: &[StreamAppendEvent],
        dict: &mut DictionaryManager,
    ) -> Result<EncodedBatch, ObsStoreError> {
        if events.is_empty() {
            return Err(ObsStoreError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "cannot seal empty CHCE batch",
            )));
        }

        let row_count = events.len() as u32;
        let mut min_ts = u64::MAX;
        let mut max_ts = 0_u64;

        let mut snapshot = Vec::with_capacity(events.len());
        let mut request_id = Vec::with_capacity(events.len());
        let mut trace_id = Vec::with_capacity(events.len());
        let mut turn_id = Vec::with_capacity(events.len());
        let mut terminal = Vec::with_capacity(events.len());
        let mut route = Vec::with_capacity(events.len());
        let mut cache_decision = Vec::with_capacity(events.len());
        let mut decision_id = Vec::with_capacity(events.len());
        let mut inference_runtime = Vec::with_capacity(events.len());
        let mut mode = Vec::with_capacity(events.len());
        let mut model = Vec::with_capacity(events.len());
        let mut elapsed_ms = Vec::with_capacity(events.len());
        let mut chunks_sent = Vec::with_capacity(events.len());
        let mut prompt_tokens = Vec::with_capacity(events.len());
        let mut context_tokens = Vec::with_capacity(events.len());
        let mut context_candidates = Vec::with_capacity(events.len());
        let mut context_selected = Vec::with_capacity(events.len());
        let mut context_truncated = Vec::with_capacity(events.len());
        let mut retrieval = Vec::with_capacity(events.len());
        let mut compression_strategy = Vec::with_capacity(events.len());
        let mut cached_tokens = Vec::with_capacity(events.len());
        let mut prefix_hash = Vec::with_capacity(events.len());
        let mut parse_retries = Vec::with_capacity(events.len());
        let mut created_at_ms = Vec::with_capacity(events.len());

        for event in events {
            let record = &event.record;
            min_ts = min_ts.min(event.created_at_ms);
            max_ts = max_ts.max(event.created_at_ms);

            snapshot.push(dict.ordinal(&record.snapshot_id)?);
            request_id.push(record.request_id);
            trace_id.push(record.trace_id.clone());
            turn_id.push(dict.ordinal(&record.turn_id)?);
            terminal.push(dict.ordinal(&record.terminal)?);
            route.push(dict.ordinal(&record.route)?);
            cache_decision.push(dict.ordinal(&record.cache_decision)?);
            decision_id.push(dict.ordinal(&record.decision_id)?);
            inference_runtime.push(dict.ordinal(&record.inference_runtime)?);
            mode.push(dict.ordinal(&record.mode)?);
            model.push(dict.ordinal(&record.model)?);
            elapsed_ms.push(record.elapsed_ms);
            chunks_sent.push(record.chunks_sent);
            prompt_tokens.push(record.prompt_tokens);
            context_tokens.push(record.context_tokens);
            context_candidates.push(record.context_candidates);
            context_selected.push(record.context_selected);
            context_truncated.push(record.context_truncated);
            retrieval.push(dict.ordinal(&record.retrieval)?);
            compression_strategy.push(dict.ordinal(&record.compression_strategy)?);
            cached_tokens.push(record.cached_tokens);
            prefix_hash.push(record.prefix_hash.clone());
            parse_retries.push(record.parse_retries);
            created_at_ms.push(event.created_at_ms);
        }

        let mut payload = Vec::new();
        write_u32(&mut payload, row_count)?;
        write_column_u16(&mut payload, COL_SNAPSHOT, &snapshot)?;
        write_column_u64(&mut payload, COL_REQUEST_ID, &request_id)?;
        write_column_strings(&mut payload, COL_TRACE_ID, &trace_id)?;
        write_column_u16(&mut payload, COL_TURN_ID, &turn_id)?;
        write_column_u16(&mut payload, COL_TERMINAL, &terminal)?;
        write_column_u16(&mut payload, COL_ROUTE, &route)?;
        write_column_u16(&mut payload, COL_CACHE_DECISION, &cache_decision)?;
        write_column_u16(&mut payload, COL_DECISION_ID, &decision_id)?;
        write_column_u16(&mut payload, COL_INFERENCE_RUNTIME, &inference_runtime)?;
        write_column_u16(&mut payload, COL_MODE, &mode)?;
        write_column_u16(&mut payload, COL_MODEL, &model)?;
        write_column_u64(&mut payload, COL_ELAPSED_MS, &elapsed_ms)?;
        write_column_u64(&mut payload, COL_CHUNKS_SENT, &chunks_sent)?;
        write_column_u64(&mut payload, COL_PROMPT_TOKENS, &prompt_tokens)?;
        write_column_u64(&mut payload, COL_CONTEXT_TOKENS, &context_tokens)?;
        write_column_u64(&mut payload, COL_CONTEXT_CANDIDATES, &context_candidates)?;
        write_column_u64(&mut payload, COL_CONTEXT_SELECTED, &context_selected)?;
        write_column_bool(&mut payload, COL_CONTEXT_TRUNCATED, &context_truncated)?;
        write_column_u16(&mut payload, COL_RETRIEVAL, &retrieval)?;
        write_column_u16(
            &mut payload,
            COL_COMPRESSION_STRATEGY,
            &compression_strategy,
        )?;
        write_column_optional_u64(&mut payload, COL_CACHED_TOKENS, &cached_tokens)?;
        write_column_optional_strings(&mut payload, COL_PREFIX_HASH, &prefix_hash)?;
        write_column_optional_u64(&mut payload, COL_PARSE_RETRIES, &parse_retries)?;
        write_column_u64(&mut payload, COL_CREATED_AT_MS, &created_at_ms)?;

        if payload.len() > MAX_PAYLOAD_SIZE {
            return Err(ObsStoreError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "CHCE batch exceeds page payload budget",
            )));
        }

        Ok(EncodedBatch {
            payload,
            min_ts,
            max_ts,
            record_count: row_count,
        })
    }

    pub fn decode_page(
        page: &PageView,
        dict: &DictionaryManager,
    ) -> Result<StreamBatch, ObsStoreError> {
        let payload_len = u32::from_le_bytes(page.data[0x18..0x1C].try_into().unwrap()) as usize;
        let payload_end = HEADER_SIZE
            .saturating_add(payload_len)
            .min(ZONE_FOOTER_OFFSET);
        let payload = &page.data[HEADER_SIZE..payload_end];
        let mut offset = 0_usize;
        let row_count = read_u32_at(payload, &mut offset)? as usize;

        let mut columns: std::collections::HashMap<u8, Vec<u8>> = std::collections::HashMap::new();
        while offset < payload.len() {
            if offset + 5 > payload.len() {
                break;
            }
            let col_id = payload[offset];
            offset += 1;
            let len = read_u32_at(payload, &mut offset)? as usize;
            if offset + len > payload.len() {
                return Err(corrupt_page("column length out of bounds"));
            }
            columns.insert(col_id, payload[offset..offset + len].to_vec());
            offset += len;
        }

        let lookup_u16 = |col_id: u8| -> Result<Vec<u16>, ObsStoreError> {
            let bytes = columns.get(&col_id).ok_or_else(|| missing_col(col_id))?;
            decode_u16_column(bytes, row_count)
        };
        let lookup_u64 = |col_id: u8| -> Result<Vec<u64>, ObsStoreError> {
            let bytes = columns.get(&col_id).ok_or_else(|| missing_col(col_id))?;
            decode_u64_column(bytes, row_count)
        };
        let lookup_strings = |col_id: u8| -> Result<Vec<String>, ObsStoreError> {
            let bytes = columns.get(&col_id).ok_or_else(|| missing_col(col_id))?;
            decode_string_column(bytes, row_count)
        };
        let lookup_bool = |col_id: u8| -> Result<Vec<bool>, ObsStoreError> {
            let bytes = columns.get(&col_id).ok_or_else(|| missing_col(col_id))?;
            decode_bool_column(bytes, row_count)
        };
        let lookup_opt_u64 = |col_id: u8| -> Result<Vec<Option<u64>>, ObsStoreError> {
            let bytes = columns.get(&col_id).ok_or_else(|| missing_col(col_id))?;
            decode_optional_u64_column(bytes, row_count)
        };
        let lookup_opt_strings = |col_id: u8| -> Result<Vec<Option<String>>, ObsStoreError> {
            let bytes = columns.get(&col_id).ok_or_else(|| missing_col(col_id))?;
            decode_optional_string_column(bytes, row_count)
        };

        let snapshot = lookup_u16(COL_SNAPSHOT)?;
        let request_id = lookup_u64(COL_REQUEST_ID)?;
        let trace_id = lookup_strings(COL_TRACE_ID)?;
        let turn_id = lookup_u16(COL_TURN_ID)?;
        let terminal = lookup_u16(COL_TERMINAL)?;
        let route = lookup_u16(COL_ROUTE)?;
        let cache_decision = lookup_u16(COL_CACHE_DECISION)?;
        let decision_id = lookup_u16(COL_DECISION_ID)?;
        let inference_runtime = lookup_u16(COL_INFERENCE_RUNTIME)?;
        let mode = lookup_u16(COL_MODE)?;
        let model = lookup_u16(COL_MODEL)?;
        let elapsed_ms = lookup_u64(COL_ELAPSED_MS)?;
        let chunks_sent = lookup_u64(COL_CHUNKS_SENT)?;
        let prompt_tokens = lookup_u64(COL_PROMPT_TOKENS)?;
        let context_tokens = lookup_u64(COL_CONTEXT_TOKENS)?;
        let context_candidates = lookup_u64(COL_CONTEXT_CANDIDATES)?;
        let context_selected = lookup_u64(COL_CONTEXT_SELECTED)?;
        let context_truncated = lookup_bool(COL_CONTEXT_TRUNCATED)?;
        let retrieval = lookup_u16(COL_RETRIEVAL)?;
        let compression_strategy = lookup_u16(COL_COMPRESSION_STRATEGY)?;
        let cached_tokens = lookup_opt_u64(COL_CACHED_TOKENS)?;
        let prefix_hash = lookup_opt_strings(COL_PREFIX_HASH)?;
        let parse_retries = lookup_opt_u64(COL_PARSE_RETRIES)?;
        let created_at_ms = lookup_u64(COL_CREATED_AT_MS)?;

        let resolve = |ordinal: u16| -> Result<String, ObsStoreError> {
            if ordinal == NULL_ORDINAL {
                return Ok(String::new());
            }
            dict.lookup(ordinal)
                .map(str::to_string)
                .ok_or_else(|| corrupt_page("unknown dictionary ordinal"))
        };

        let mut records = Vec::with_capacity(row_count);
        for i in 0..row_count {
            records.push(StreamEconomicsRecord {
                snapshot_id: resolve(snapshot[i])?,
                request_id: request_id[i],
                trace_id: trace_id[i].clone(),
                turn_id: resolve(turn_id[i])?,
                terminal: resolve(terminal[i])?,
                route: resolve(route[i])?,
                cache_decision: resolve(cache_decision[i])?,
                decision_id: resolve(decision_id[i])?,
                inference_runtime: resolve(inference_runtime[i])?,
                mode: resolve(mode[i])?,
                model: resolve(model[i])?,
                elapsed_ms: elapsed_ms[i],
                chunks_sent: chunks_sent[i],
                prompt_tokens: prompt_tokens[i],
                context_tokens: context_tokens[i],
                context_candidates: context_candidates[i],
                context_selected: context_selected[i],
                context_truncated: context_truncated[i],
                retrieval: resolve(retrieval[i])?,
                compression_strategy: resolve(compression_strategy[i])?,
                cached_tokens: cached_tokens[i],
                prefix_hash: prefix_hash[i].clone(),
                parse_retries: parse_retries[i],
            });
        }

        Ok(StreamBatch {
            records,
            created_at_ms,
        })
    }
}

/// One sealed 16 KB page view.
#[derive(Debug, Clone)]
pub struct PageView {
    #[allow(dead_code)]
    pub block_seq: u64,
    pub data: Vec<u8>,
}

/// Sealed page bytes ready for append.
#[derive(Debug, Clone)]
pub struct SealedPage {
    pub bytes: Vec<u8>,
    #[allow(dead_code)]
    pub record_count: u32,
}

impl SealedPage {
    pub fn seal(batch: EncodedBatch) -> Result<Self, ObsStoreError> {
        let mut page = vec![0_u8; PAGE_SIZE];
        page[0..4].copy_from_slice(&MAGIC);
        page[4..6].copy_from_slice(&FORMAT_VERSION.to_le_bytes());
        page[6..8].copy_from_slice(&(PAGE_SIZE as u16).to_le_bytes());

        let payload_len = batch.payload.len();
        page[HEADER_SIZE..HEADER_SIZE + payload_len].copy_from_slice(&batch.payload);

        page[0x18..0x1C].copy_from_slice(&(batch.payload.len() as u32).to_le_bytes());
        let crc = crc32(&page[HEADER_SIZE..ZONE_FOOTER_OFFSET]);
        page[0x10..0x14].copy_from_slice(&crc.to_le_bytes());

        page[ZONE_FOOTER_OFFSET..ZONE_FOOTER_OFFSET + 8]
            .copy_from_slice(&batch.min_ts.to_le_bytes());
        page[ZONE_FOOTER_OFFSET + 8..ZONE_FOOTER_OFFSET + 16]
            .copy_from_slice(&batch.max_ts.to_le_bytes());
        page[ZONE_FOOTER_OFFSET + 16..ZONE_FOOTER_OFFSET + 20]
            .copy_from_slice(&batch.record_count.to_le_bytes());

        Ok(Self {
            bytes: page,
            record_count: batch.record_count,
        })
    }
}

/// Mmap-backed paginator for `store.rexobs`.
pub struct MmapPaginator {
    path: PathBuf,
    committed_byte_len: u64,
    next_block_seq: u64,
}

impl MmapPaginator {
    pub fn open(path: PathBuf) -> Result<Self, ObsStoreError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)?;
        let metadata = file.metadata()?;
        let file_len = metadata.len();

        let mut paginator = Self {
            path,
            committed_byte_len: 0,
            next_block_seq: 0,
        };

        if file_len >= COMMIT_TAIL_SIZE {
            paginator.recover(file_len)?;
        } else {
            file.set_len(COMMIT_TAIL_SIZE)?;
            paginator.write_commit_tail(0)?;
        }
        Ok(paginator)
    }

    pub fn append_page(&mut self, page: &SealedPage) -> Result<(), ObsStoreError> {
        let mut file = OpenOptions::new().read(true).write(true).open(&self.path)?;

        let page_offset = self.committed_byte_len;
        let mut page_bytes = page.bytes.clone();
        page_bytes[0x08..0x10].copy_from_slice(&self.next_block_seq.to_le_bytes());

        file.seek(SeekFrom::Start(page_offset))?;
        file.write_all(&page_bytes)?;

        let new_committed = page_offset + PAGE_SIZE as u64;
        file.set_len(new_committed + COMMIT_TAIL_SIZE)?;
        self.write_commit_tail_to(&mut file, new_committed)?;
        file.sync_data()?;

        self.committed_byte_len = new_committed;
        self.next_block_seq += 1;
        Ok(())
    }

    pub fn total_record_count(&self) -> Result<u64, ObsStoreError> {
        let mut total = 0_u64;
        for page in self.iter_valid_pages()? {
            let count = u32::from_le_bytes(
                page.data[ZONE_FOOTER_OFFSET + 16..ZONE_FOOTER_OFFSET + 20]
                    .try_into()
                    .unwrap(),
            );
            total += count as u64;
        }
        Ok(total)
    }

    pub fn iter_valid_pages(&self) -> Result<Vec<PageView>, ObsStoreError> {
        let file = File::open(&self.path)?;
        let map = unsafe { MmapOptions::new().map(&file)? };
        let mut pages = Vec::new();
        let mut offset = 0_u64;
        while offset + PAGE_SIZE as u64 <= self.committed_byte_len {
            let start = offset as usize;
            let end = start + PAGE_SIZE;
            let slice = &map[start..end];
            if slice[0..4] != MAGIC {
                break;
            }
            let format_ver = u16::from_le_bytes(slice[4..6].try_into().unwrap());
            if format_ver != FORMAT_VERSION {
                return Err(ObsStoreError::FormatVersionUnsupported {
                    version: format_ver,
                });
            }
            let stored_crc = u32::from_le_bytes(slice[0x10..0x14].try_into().unwrap());
            let actual_crc = crc32(&slice[HEADER_SIZE..ZONE_FOOTER_OFFSET]);
            if stored_crc != actual_crc {
                break;
            }
            let block_seq = u64::from_le_bytes(slice[0x08..0x10].try_into().unwrap());
            pages.push(PageView {
                block_seq,
                data: slice.to_vec(),
            });
            offset += PAGE_SIZE as u64;
        }
        Ok(pages)
    }

    #[allow(dead_code)]
    pub fn committed_byte_len(&self) -> u64 {
        self.committed_byte_len
    }

    pub fn recover_from_torn_page(&mut self, valid_page_count: u64) -> Result<(), ObsStoreError> {
        let new_committed = valid_page_count * PAGE_SIZE as u64;
        let file = OpenOptions::new().write(true).open(&self.path)?;
        file.set_len(new_committed + COMMIT_TAIL_SIZE)?;
        self.write_commit_tail(new_committed)?;
        self.committed_byte_len = new_committed;
        self.next_block_seq = valid_page_count;
        Ok(())
    }

    fn recover(&mut self, file_len: u64) -> Result<(), ObsStoreError> {
        let file = File::open(&self.path)?;
        let map = unsafe { MmapOptions::new().map(&file)? };
        if file_len < COMMIT_TAIL_SIZE {
            return Err(ObsStoreError::RecoveryFailed);
        }
        let tail_offset = (file_len - COMMIT_TAIL_SIZE) as usize;
        let committed = u64::from_le_bytes(map[tail_offset..tail_offset + 8].try_into().unwrap());

        let mut valid_pages = 0_u64;
        let mut offset = 0_u64;
        while offset + PAGE_SIZE as u64 <= committed {
            let start = offset as usize;
            let page = &map[start..start + PAGE_SIZE];
            if page[0..4] != MAGIC {
                break;
            }
            let format_ver = u16::from_le_bytes(page[4..6].try_into().unwrap());
            if format_ver != FORMAT_VERSION {
                return Err(ObsStoreError::FormatVersionUnsupported {
                    version: format_ver,
                });
            }
            let stored_crc = u32::from_le_bytes(page[0x10..0x14].try_into().unwrap());
            let actual_crc = crc32(&page[HEADER_SIZE..ZONE_FOOTER_OFFSET]);
            if stored_crc != actual_crc {
                break;
            }
            valid_pages += 1;
            offset += PAGE_SIZE as u64;
        }

        if valid_pages == 0 && committed > 0 {
            return Err(ObsStoreError::RecoveryFailed);
        }

        let recovered_committed = valid_pages * PAGE_SIZE as u64;
        if recovered_committed != committed {
            self.recover_from_torn_page(valid_pages)?;
        } else {
            self.committed_byte_len = committed;
            self.next_block_seq = valid_pages;
        }
        Ok(())
    }

    fn write_commit_tail(&self, committed: u64) -> Result<(), ObsStoreError> {
        let mut file = OpenOptions::new().write(true).open(&self.path)?;
        self.write_commit_tail_to(&mut file, committed)
    }

    fn write_commit_tail_to(&self, file: &mut File, committed: u64) -> Result<(), ObsStoreError> {
        file.seek(SeekFrom::Start(committed))?;
        file.write_all(&committed.to_le_bytes())?;
        Ok(())
    }
}

fn crc32(data: &[u8]) -> u32 {
    let mut hasher = Crc32Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

fn write_u32(buf: &mut Vec<u8>, value: u32) -> Result<(), ObsStoreError> {
    buf.extend_from_slice(&value.to_le_bytes());
    Ok(())
}

fn write_column_u16(buf: &mut Vec<u8>, col_id: u8, values: &[u16]) -> Result<(), ObsStoreError> {
    buf.push(col_id);
    let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
    write_u32(buf, bytes.len() as u32)?;
    buf.extend_from_slice(&bytes);
    Ok(())
}

fn write_column_u64(buf: &mut Vec<u8>, col_id: u8, values: &[u64]) -> Result<(), ObsStoreError> {
    buf.push(col_id);
    let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
    write_u32(buf, bytes.len() as u32)?;
    buf.extend_from_slice(&bytes);
    Ok(())
}

fn write_column_bool(buf: &mut Vec<u8>, col_id: u8, values: &[bool]) -> Result<(), ObsStoreError> {
    buf.push(col_id);
    let bytes: Vec<u8> = values.iter().map(|v| u8::from(*v)).collect();
    write_u32(buf, bytes.len() as u32)?;
    buf.extend_from_slice(&bytes);
    Ok(())
}

fn write_column_strings(
    buf: &mut Vec<u8>,
    col_id: u8,
    values: &[String],
) -> Result<(), ObsStoreError> {
    buf.push(col_id);
    let mut bytes = Vec::new();
    for value in values {
        let len = value.len() as u32;
        bytes.extend_from_slice(&len.to_le_bytes());
        bytes.extend_from_slice(value.as_bytes());
    }
    write_u32(buf, bytes.len() as u32)?;
    buf.extend_from_slice(&bytes);
    Ok(())
}

fn write_column_optional_u64(
    buf: &mut Vec<u8>,
    col_id: u8,
    values: &[Option<u64>],
) -> Result<(), ObsStoreError> {
    buf.push(col_id);
    let mut bytes = Vec::new();
    let bitmap = optional_bitmap(values);
    bytes.extend(bitmap);
    for value in values.iter().flatten() {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    write_u32(buf, bytes.len() as u32)?;
    buf.extend_from_slice(&bytes);
    Ok(())
}

fn write_column_optional_strings(
    buf: &mut Vec<u8>,
    col_id: u8,
    values: &[Option<String>],
) -> Result<(), ObsStoreError> {
    buf.push(col_id);
    let mut bytes = Vec::new();
    let bitmap = optional_bitmap(values);
    bytes.extend(bitmap);
    for value in values.iter().flatten() {
        let value: &String = value;
        let len = value.len() as u32;
        bytes.extend_from_slice(&len.to_le_bytes());
        bytes.extend_from_slice(value.as_bytes());
    }
    write_u32(buf, bytes.len() as u32)?;
    buf.extend_from_slice(&bytes);
    Ok(())
}

fn optional_bitmap<T>(values: &[Option<T>]) -> Vec<u8> {
    let byte_len = values.len().div_ceil(8);
    let mut bitmap = vec![0_u8; byte_len];
    for (idx, value) in values.iter().enumerate() {
        if value.is_some() {
            bitmap[idx / 8] |= 1 << (idx % 8);
        }
    }
    bitmap
}

fn read_u32_at(buf: &[u8], offset: &mut usize) -> Result<u32, ObsStoreError> {
    if *offset + 4 > buf.len() {
        return Err(corrupt_page("unexpected end of payload"));
    }
    let value = u32::from_le_bytes(buf[*offset..*offset + 4].try_into().unwrap());
    *offset += 4;
    Ok(value)
}

fn decode_u16_column(bytes: &[u8], row_count: usize) -> Result<Vec<u16>, ObsStoreError> {
    if bytes.len() != row_count * 2 {
        return Err(corrupt_page("u16 column length mismatch"));
    }
    Ok(bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes(chunk.try_into().unwrap()))
        .collect())
}

fn decode_u64_column(bytes: &[u8], row_count: usize) -> Result<Vec<u64>, ObsStoreError> {
    if bytes.len() != row_count * 8 {
        return Err(corrupt_page("u64 column length mismatch"));
    }
    Ok(bytes
        .chunks_exact(8)
        .map(|chunk| u64::from_le_bytes(chunk.try_into().unwrap()))
        .collect())
}

fn decode_bool_column(bytes: &[u8], row_count: usize) -> Result<Vec<bool>, ObsStoreError> {
    if bytes.len() != row_count {
        return Err(corrupt_page("bool column length mismatch"));
    }
    Ok(bytes.iter().map(|b| *b != 0).collect())
}

fn decode_string_column(bytes: &[u8], row_count: usize) -> Result<Vec<String>, ObsStoreError> {
    let mut offset = 0_usize;
    let mut out = Vec::with_capacity(row_count);
    for _ in 0..row_count {
        if offset + 4 > bytes.len() {
            return Err(corrupt_page("string column truncated"));
        }
        let len = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        if offset + len > bytes.len() {
            return Err(corrupt_page("string payload truncated"));
        }
        let value = std::str::from_utf8(&bytes[offset..offset + len])
            .map_err(|_| corrupt_page("invalid UTF-8 in string column"))?;
        out.push(value.to_string());
        offset += len;
    }
    Ok(out)
}

fn decode_optional_u64_column(
    bytes: &[u8],
    row_count: usize,
) -> Result<Vec<Option<u64>>, ObsStoreError> {
    let bitmap_len = row_count.div_ceil(8);
    if bytes.len() < bitmap_len {
        return Err(corrupt_page("optional u64 bitmap truncated"));
    }
    let bitmap = &bytes[..bitmap_len];
    let mut offset = bitmap_len;
    let mut out = Vec::with_capacity(row_count);
    for idx in 0..row_count {
        if bitmap[idx / 8] & (1 << (idx % 8)) != 0 {
            if offset + 8 > bytes.len() {
                return Err(corrupt_page("optional u64 value truncated"));
            }
            let value = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
            out.push(Some(value));
            offset += 8;
        } else {
            out.push(None);
        }
    }
    Ok(out)
}

fn decode_optional_string_column(
    bytes: &[u8],
    row_count: usize,
) -> Result<Vec<Option<String>>, ObsStoreError> {
    let bitmap_len = row_count.div_ceil(8);
    if bytes.len() < bitmap_len {
        return Err(corrupt_page("optional string bitmap truncated"));
    }
    let bitmap = &bytes[..bitmap_len];
    let mut offset = bitmap_len;
    let mut out = Vec::with_capacity(row_count);
    for idx in 0..row_count {
        if bitmap[idx / 8] & (1 << (idx % 8)) != 0 {
            if offset + 4 > bytes.len() {
                return Err(corrupt_page("optional string length truncated"));
            }
            let len = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;
            if offset + len > bytes.len() {
                return Err(corrupt_page("optional string payload truncated"));
            }
            let value = std::str::from_utf8(&bytes[offset..offset + len])
                .map_err(|_| corrupt_page("invalid UTF-8 in optional string column"))?;
            out.push(Some(value.to_string()));
            offset += len;
        } else {
            out.push(None);
        }
    }
    Ok(out)
}

fn missing_col(col_id: u8) -> ObsStoreError {
    ObsStoreError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("missing CHCE column {col_id}"),
    ))
}

fn corrupt_page(message: &str) -> ObsStoreError {
    ObsStoreError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        message,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chce::ring::StreamAppendEvent;
    use tempfile::tempdir;

    fn sample_event(request_id: u64) -> StreamAppendEvent {
        StreamAppendEvent {
            record: StreamEconomicsRecord {
                snapshot_id: "snap".to_string(),
                request_id,
                trace_id: format!("trace-{request_id}"),
                turn_id: "turn-1".to_string(),
                terminal: "done".to_string(),
                route: "sidecar+mock".to_string(),
                cache_decision: "miss_stored".to_string(),
                decision_id: format!("dec-{request_id}"),
                inference_runtime: "mock".to_string(),
                mode: "ask".to_string(),
                model: "gpt-4o-mini".to_string(),
                elapsed_ms: 42,
                chunks_sent: 3,
                prompt_tokens: 100,
                context_tokens: 50,
                context_candidates: 10,
                context_selected: 5,
                context_truncated: false,
                retrieval: "skipped".to_string(),
                compression_strategy: "extractive_query".to_string(),
                cached_tokens: Some(12),
                prefix_hash: Some("abc".to_string()),
                parse_retries: Some(1),
            },
            created_at_ms: 1_700_000_000_000 + request_id,
        }
    }

    #[test]
    fn sealed_page_has_rexo_layout() {
        let dir = tempdir().unwrap();
        let mut dict = DictionaryManager::open(dir.path().join("store.dict")).unwrap();
        let batch = ColumnarCodec::encode_batch(&[sample_event(1)], &mut dict).unwrap();
        let page = SealedPage::seal(batch).unwrap();
        assert_eq!(page.bytes.len(), PAGE_SIZE);
        assert_eq!(&page.bytes[0..4], &MAGIC);
        assert_eq!(
            u16::from_le_bytes(page.bytes[4..6].try_into().unwrap()),
            FORMAT_VERSION
        );
        assert_eq!(
            u16::from_le_bytes(page.bytes[6..8].try_into().unwrap()),
            PAGE_SIZE as u16
        );
    }

    #[test]
    fn columnar_round_trip_matches_fixture_fields() {
        let dir = tempdir().unwrap();
        let mut dict = DictionaryManager::open(dir.path().join("store.dict")).unwrap();
        let events = vec![sample_event(1), sample_event(2)];
        let batch = ColumnarCodec::encode_batch(&events, &mut dict).unwrap();
        let page = SealedPage::seal(batch).unwrap();
        let view = PageView {
            block_seq: 0,
            data: page.bytes,
        };
        let decoded = ColumnarCodec::decode_page(&view, &dict).unwrap();
        assert_eq!(
            decoded.records,
            events.iter().map(|e| e.record.clone()).collect::<Vec<_>>()
        );
    }
}

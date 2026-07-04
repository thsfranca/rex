//! Retroactive / incremental session event fetch (daemon SoT).

use rex_proto::rex::v1::FetchSessionEventsRequest;
use tokio::sync::mpsc;

use crate::error::CliError;
use crate::harness_session;
use crate::transport::connect_client;

use super::viewport::DEFAULT_FETCH_LIMIT;

pub enum HistoryFetchUpdate {
    Retroactive {
        events: Vec<rex_proto::rex::v1::SessionEvent>,
        has_more_before: bool,
        head_sequence: u64,
    },
    Incremental {
        events: Vec<rex_proto::rex::v1::SessionEvent>,
        has_more_after: bool,
        head_sequence: u64,
    },
    Failed(String),
}

pub async fn spawn_retroactive_fetch(
    harness_session_id: String,
    before_sequence: u64,
    limit: u32,
) -> Result<mpsc::Receiver<HistoryFetchUpdate>, CliError> {
    spawn_fetch(
        harness_session_id,
        before_sequence,
        0,
        limit,
        HistoryFetchKind::Retroactive,
    )
    .await
}

pub async fn spawn_incremental_fetch(
    harness_session_id: String,
    after_sequence: u64,
    limit: u32,
) -> Result<mpsc::Receiver<HistoryFetchUpdate>, CliError> {
    spawn_fetch(
        harness_session_id,
        0,
        after_sequence,
        limit,
        HistoryFetchKind::Incremental,
    )
    .await
}

enum HistoryFetchKind {
    Retroactive,
    Incremental,
}

async fn spawn_fetch(
    harness_session_id: String,
    before_sequence: u64,
    after_sequence: u64,
    limit: u32,
    kind: HistoryFetchKind,
) -> Result<mpsc::Receiver<HistoryFetchUpdate>, CliError> {
    let (tx, rx) = mpsc::channel(8);
    tokio::spawn(async move {
        let result = run_fetch(
            &harness_session_id,
            before_sequence,
            after_sequence,
            limit,
        )
        .await;
        match result {
            Ok(page) => {
                let msg = match kind {
                    HistoryFetchKind::Retroactive => HistoryFetchUpdate::Retroactive {
                        events: page.events,
                        has_more_before: page.has_more_before,
                        head_sequence: page.head_sequence,
                    },
                    HistoryFetchKind::Incremental => HistoryFetchUpdate::Incremental {
                        events: page.events,
                        has_more_after: page.has_more_after,
                        head_sequence: page.head_sequence,
                    },
                };
                let _ = tx.send(msg).await;
            }
            Err(err) => {
                let _ = tx
                    .send(HistoryFetchUpdate::Failed(err.to_string()))
                    .await;
            }
        }
    });
    Ok(rx)
}

struct FetchPage {
    events: Vec<rex_proto::rex::v1::SessionEvent>,
    has_more_before: bool,
    has_more_after: bool,
    head_sequence: u64,
}

async fn run_fetch(
    harness_session_id: &str,
    before_sequence: u64,
    after_sequence: u64,
    limit: u32,
) -> Result<FetchPage, CliError> {
    let mut client = connect_client(None).await?;
    let mut request = tonic::Request::new(FetchSessionEventsRequest {
        harness_session_id: harness_session_id.to_string(),
        before_sequence,
        after_sequence,
        limit: if limit == 0 {
            DEFAULT_FETCH_LIMIT
        } else {
            limit
        },
    });
    harness_session::insert_metadata(request.metadata_mut(), harness_session_id)
        .map_err(CliError::Status)?;
    let response = client.fetch_session_events(request).await?;
    let inner = response.into_inner();
    Ok(FetchPage {
        events: inner.events,
        has_more_before: inner.has_more_before,
        has_more_after: inner.has_more_after,
        head_sequence: inner.head_sequence,
    })
}

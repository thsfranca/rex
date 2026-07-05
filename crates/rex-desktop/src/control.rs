use std::time::Duration;

use rex_cli::transport::connect_client;
use rex_cli::{insert_harness_session_metadata, system_status, CliError};
use rex_proto::rex::v1::{
    FetchSessionEventsRequest, FetchSessionEventsResponse, RespondToToolApprovalRequest,
    RespondToToolApprovalResponse,
};
use serde::Serialize;
use tonic::Request;

const REQUEST_TIMEOUT_SECONDS: u64 = 5;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStatusDto {
    pub daemon_version: String,
    pub uptime_seconds: u64,
    pub active_model_id: String,
    pub workspace_root: String,
    pub lifecycle_state: String,
    pub idle_seconds: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEventDto {
    pub sequence: u64,
    pub event: String,
    pub text: String,
    pub turn_id: String,
    pub tool_name: String,
    pub phase: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchSessionEventsDto {
    pub events: Vec<SessionEventDto>,
    pub has_more_before: bool,
    pub has_more_after: bool,
    pub head_sequence: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolApprovalResultDto {
    pub ok: bool,
    pub error: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DaemonLifecycleEvent {
    Ready { workspace_root: String },
    Unavailable { message: String },
}

pub async fn get_system_status() -> Result<SystemStatusDto, CliError> {
    let status = system_status().await?;
    Ok(SystemStatusDto {
        daemon_version: status.daemon_version,
        uptime_seconds: status.uptime_seconds,
        active_model_id: status.active_model_id,
        workspace_root: status.workspace_root,
        lifecycle_state: status.lifecycle_state,
        idle_seconds: status.idle_seconds,
    })
}

pub async fn fetch_session_events(
    harness_session_id: String,
    before_sequence: u64,
    after_sequence: u64,
    limit: u32,
) -> Result<FetchSessionEventsDto, CliError> {
    let mut client = connect_client(None).await?;
    let mut request = Request::new(FetchSessionEventsRequest {
        harness_session_id,
        before_sequence,
        after_sequence,
        limit,
    });
    request.set_timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS));
    let response = client.fetch_session_events(request).await?;
    Ok(map_fetch_response(response.into_inner()))
}

pub async fn respond_to_tool_approval(
    approval_token: String,
    approved: bool,
    tool_call_id: String,
    harness_session_id: String,
) -> Result<ToolApprovalResultDto, CliError> {
    let mut client = connect_client(None).await?;
    let mut request = Request::new(RespondToToolApprovalRequest {
        approval_token,
        approved,
        tool_call_id,
    });
    request.set_timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS));
    insert_harness_session_metadata(request.metadata_mut(), &harness_session_id)
        .map_err(CliError::Status)?;
    let response = client.respond_to_tool_approval(request).await?;
    Ok(map_approval_response(response.into_inner()))
}

fn map_fetch_response(response: FetchSessionEventsResponse) -> FetchSessionEventsDto {
    FetchSessionEventsDto {
        events: response
            .events
            .into_iter()
            .map(|e| SessionEventDto {
                sequence: e.sequence,
                event: e.event,
                text: e.text,
                turn_id: e.turn_id,
                tool_name: e.tool_name,
                phase: e.phase,
            })
            .collect(),
        has_more_before: response.has_more_before,
        has_more_after: response.has_more_after,
        head_sequence: response.head_sequence,
    }
}

fn map_approval_response(response: RespondToToolApprovalResponse) -> ToolApprovalResultDto {
    ToolApprovalResultDto {
        ok: response.ok,
        error: response.error,
    }
}

pub async fn probe_daemon_lifecycle() -> DaemonLifecycleEvent {
    match get_system_status().await {
        Ok(status) => DaemonLifecycleEvent::Ready {
            workspace_root: status.workspace_root,
        },
        Err(err) => DaemonLifecycleEvent::Unavailable {
            message: err.operator_message(),
        },
    }
}

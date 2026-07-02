//! Unary gRPC helper for tool approval responses.

use rex_proto::rex::v1::RespondToToolApprovalRequest;
use tonic::Request;

use crate::domain::REQUEST_TIMEOUT_SECONDS;
use crate::error::CliError;
use crate::transport::connect_client;

pub async fn respond_to_tool_approval(
    approval_token: &str,
    tool_call_id: &str,
    approved: bool,
) -> Result<(), CliError> {
    let mut client = connect_client(None).await?;
    let mut request = Request::new(RespondToToolApprovalRequest {
        approval_token: approval_token.to_string(),
        approved,
        tool_call_id: tool_call_id.to_string(),
    });
    request.set_timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECONDS));
    let response = client.respond_to_tool_approval(request).await?;
    let inner = response.into_inner();
    if inner.ok {
        Ok(())
    } else {
        Err(CliError::Status(tonic::Status::failed_precondition(
            inner.error,
        )))
    }
}

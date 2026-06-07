//! Sidecar `BrokerInference` orchestration (R038 native tools gate + HTTP dispatch).

use rex_config::NativeToolsMode;
use rex_proto::rex::v1::{
    BrokerInferenceRequest, BrokerInferenceResponse, InferenceProtocol, ToolCall, ToolSpec,
};

use crate::adapters::RuntimeKind;
use crate::http_openai_compat::{
    broker_inference_http, BrokerCompletionResult, HttpChatMessage, HttpToolSpec,
};

pub fn normalize_messages(request: &BrokerInferenceRequest) -> Vec<HttpChatMessage> {
    if !request.messages.is_empty() {
        return request
            .messages
            .iter()
            .map(|m| HttpChatMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();
    }
    let prompt = request.prompt.trim();
    if prompt.is_empty() {
        return Vec::new();
    }
    vec![HttpChatMessage {
        role: "user".to_string(),
        content: request.prompt.clone(),
    }]
}

pub fn request_has_tools(request: &BrokerInferenceRequest) -> bool {
    !request.tools.is_empty()
}

pub fn should_forward_native_tools(
    request: &BrokerInferenceRequest,
    native_tools: NativeToolsMode,
    runtime: RuntimeKind,
    _base_url: &str,
) -> bool {
    if !request_has_tools(request) {
        return false;
    }
    match runtime {
        RuntimeKind::Mock | RuntimeKind::CursorCli => return false,
        RuntimeKind::HttpOpenAiCompat => {}
    }
    !matches!(native_tools, NativeToolsMode::False)
}

fn http_tools_from_proto(tools: &[ToolSpec]) -> Vec<HttpToolSpec> {
    tools
        .iter()
        .map(|t| HttpToolSpec {
            name: t.name.clone(),
            description: t.description.clone(),
            parameters_json: t.parameters_json.clone(),
        })
        .collect()
}

fn proto_tool_calls(calls: &[crate::http_openai_compat::HttpToolCall]) -> Vec<ToolCall> {
    calls
        .iter()
        .map(|c| ToolCall {
            id: c.id.clone(),
            name: c.name.clone(),
            arguments_json: c.arguments_json.clone(),
        })
        .collect()
}

pub async fn run_broker_inference(
    request: &BrokerInferenceRequest,
) -> Result<BrokerInferenceResponse, tonic::Status> {
    let loaded = crate::settings::get();
    let cfg = &loaded.effective;
    let runtime = RuntimeKind::from_config();
    let base_url = loaded.effective_openai_compat_base_url();
    let native_tools = cfg.inference.openai_compat.effective_native_tools();
    let messages = normalize_messages(request);
    if messages.is_empty() {
        return Ok(error_response(
            "broker inference requires prompt or messages",
            InferenceProtocol::Interim,
        ));
    }

    let forward_tools = should_forward_native_tools(request, native_tools, runtime, &base_url);
    let tools = if forward_tools {
        http_tools_from_proto(&request.tools)
    } else {
        Vec::new()
    };

    let result = broker_inference_http(
        &messages,
        &tools,
        &request.model,
        forward_tools,
        native_tools,
        runtime,
        &base_url,
    )
    .await?;

    Ok(map_completion_to_response(result))
}

fn map_completion_to_response(result: BrokerCompletionResult) -> BrokerInferenceResponse {
    if let Some(err) = result.error {
        return BrokerInferenceResponse {
            ok: false,
            text: String::new(),
            content: String::new(),
            error: err,
            tool_calls: proto_tool_calls(&result.tool_calls),
            protocol: result.protocol as i32,
        };
    }

    let has_tool_calls = !result.tool_calls.is_empty();
    let content = result.content.clone();
    let text = if has_tool_calls {
        String::new()
    } else {
        content.clone()
    };

    BrokerInferenceResponse {
        ok: true,
        text,
        content,
        error: String::new(),
        tool_calls: proto_tool_calls(&result.tool_calls),
        protocol: result.protocol as i32,
    }
}

fn error_response(message: &str, protocol: InferenceProtocol) -> BrokerInferenceResponse {
    BrokerInferenceResponse {
        ok: false,
        text: String::new(),
        content: String::new(),
        error: message.to_string(),
        tool_calls: Vec::new(),
        protocol: protocol as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rex_proto::rex::v1::{BrokerInferenceRequest, ChatMessage};

    #[test]
    fn normalizes_prompt_fallback() {
        let req = BrokerInferenceRequest {
            prompt: "hello".to_string(),
            mode: "ask".to_string(),
            model: String::new(),
            messages: Vec::new(),
            tools: Vec::new(),
        };
        let msgs = normalize_messages(&req);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[0].content, "hello");
    }

    #[test]
    fn mock_runtime_blocks_tool_forward() {
        let req = BrokerInferenceRequest {
            prompt: String::new(),
            mode: "plan".to_string(),
            model: String::new(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: "x".to_string(),
            }],
            tools: vec![ToolSpec {
                name: "fs.read".to_string(),
                description: String::new(),
                parameters_json: "{}".to_string(),
            }],
        };
        assert!(!should_forward_native_tools(
            &req,
            NativeToolsMode::Auto,
            RuntimeKind::Mock,
            "http://127.0.0.1:11434/v1"
        ));
    }
}

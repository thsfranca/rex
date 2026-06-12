//! R038 PR1: BrokerInference native tools (daemon HTTP path).

use std::sync::Arc;

use rex_config::{NativeToolsMode, RexConfig};
use rex_proto::rex::v1::{BrokerInferenceRequest, InferenceProtocol, ToolSpec};
use serial_test::serial;

#[path = "../src/settings.rs"]
#[allow(dead_code)]
mod settings;
mod support;

use support::openai_compat_sse::{
    spawn_loopback_openai_compat_sse_fixture, spawn_loopback_openai_compat_tool_calls_fixture,
    spawn_loopback_openai_compat_tool_calls_capture_fixture,
    spawn_loopback_openai_compat_tool_reject_fixture,
};

#[allow(dead_code)]
#[path = "../src/adapters.rs"]
mod adapters;
#[allow(dead_code)]
#[path = "../src/broker_inference.rs"]
mod broker_inference;
#[allow(dead_code)]
#[path = "../src/domain.rs"]
mod domain;
#[allow(dead_code)]
#[path = "../src/http_openai_compat.rs"]
mod http_openai_compat;
#[allow(dead_code)]
#[path = "../src/ollama_capability.rs"]
mod ollama_capability;

fn init_http_config(base_url: &str, native_tools: NativeToolsMode) {
    settings::reset_for_test();
    let mut cfg = RexConfig::defaults();
    cfg.inference.runtime = "http-openai-compat".to_string();
    cfg.inference.openai_compat.base_url = base_url.to_string();
    cfg.inference.openai_compat.native_tools = Some(native_tools);
    settings::init_for_test(Arc::new(rex_config::LoadedConfig {
        rex_root: std::path::PathBuf::from("/tmp/rex-broker-native-test"),
        global_path: None,
        project_path: None,
        effective: cfg,
    }));
}

#[tokio::test]
#[serial]
async fn prompt_only_broker_inference_keeps_text_compat() {
    let addr = spawn_loopback_openai_compat_sse_fixture().await;
    init_http_config(&format!("http://{addr}/v1"), NativeToolsMode::Auto);

    let response = broker_inference::run_broker_inference(&BrokerInferenceRequest {
        prompt: "hello".to_string(),
        mode: "ask".to_string(),
        model: String::new(),
        messages: Vec::new(),
        tools: Vec::new(),
    })
    .await
    .expect("broker inference");

    assert!(response.ok, "{}", response.error);
    assert_eq!(response.text, "hello stub");
    assert_eq!(response.protocol, InferenceProtocol::Interim as i32);

    settings::reset_for_test();
}

#[tokio::test]
#[serial]
async fn native_tool_calls_returned_when_enabled() {
    let addr = spawn_loopback_openai_compat_tool_calls_fixture().await;
    init_http_config(&format!("http://{addr}/v1"), NativeToolsMode::True);

    let response = broker_inference::run_broker_inference(&BrokerInferenceRequest {
        prompt: String::new(),
        mode: "plan".to_string(),
        model: String::new(),
        messages: vec![rex_proto::rex::v1::ChatMessage {
            role: "user".to_string(),
            content: "read README".to_string(),
        }],
        tools: vec![ToolSpec {
            name: "fs.read".to_string(),
            description: "Read a file".to_string(),
            parameters_json: r#"{"type":"object","properties":{"path":{"type":"string"}}}"#
                .to_string(),
        }],
    })
    .await
    .expect("broker inference");

    assert!(response.ok, "{}", response.error);
    assert_eq!(response.protocol, InferenceProtocol::Native as i32);
    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(response.tool_calls[0].name, "fs.read");
    assert!(response.tool_calls[0].arguments_json.contains("README.md"));

    settings::reset_for_test();
}

#[tokio::test]
#[serial]
async fn mock_runtime_strips_tools_and_uses_interim() {
    let addr = spawn_loopback_openai_compat_sse_fixture().await;
    settings::reset_for_test();
    let mut cfg = RexConfig::defaults();
    cfg.inference.runtime = "mock".to_string();
    cfg.inference.openai_compat.base_url = format!("http://{addr}/v1");
    cfg.inference.openai_compat.native_tools = Some(NativeToolsMode::True);
    settings::init_for_test(Arc::new(rex_config::LoadedConfig {
        rex_root: std::path::PathBuf::from("/tmp/rex-broker-mock-test"),
        global_path: None,
        project_path: None,
        effective: cfg,
    }));

    let response = broker_inference::run_broker_inference(&BrokerInferenceRequest {
        prompt: "hello".to_string(),
        mode: "plan".to_string(),
        model: String::new(),
        messages: Vec::new(),
        tools: vec![ToolSpec {
            name: "fs.read".to_string(),
            description: String::new(),
            parameters_json: "{}".to_string(),
        }],
    })
    .await
    .expect("broker inference");

    assert!(response.ok, "{}", response.error);
    assert_eq!(response.protocol, InferenceProtocol::Interim as i32);
    assert!(response.tool_calls.is_empty());

    settings::reset_for_test();
}

#[tokio::test]
#[serial]
async fn native_tool_calls_encode_wire_names_on_request() {
    let (addr, rx) = spawn_loopback_openai_compat_tool_calls_capture_fixture().await;
    init_http_config(&format!("http://{addr}/v1"), NativeToolsMode::True);

    let response = broker_inference::run_broker_inference(&BrokerInferenceRequest {
        prompt: String::new(),
        mode: "plan".to_string(),
        model: String::new(),
        messages: vec![rex_proto::rex::v1::ChatMessage {
            role: "user".to_string(),
            content: "read README".to_string(),
        }],
        tools: vec![ToolSpec {
            name: "fs.read".to_string(),
            description: "Read a file".to_string(),
            parameters_json: r#"{"type":"object","properties":{"path":{"type":"string"}}}"#
                .to_string(),
        }],
    })
    .await
    .expect("broker inference");

    assert!(response.ok, "{}", response.error);
    assert_eq!(response.tool_calls[0].name, "fs.read");

    let request = rx.await.expect("request captured");
    assert!(
        request.contains(r#""name":"fs_read""#),
        "expected wire-encoded tool name in request, got: {request}"
    );
    assert!(
        !request.contains(r#""name":"fs.read""#),
        "canonical dotted name must not appear on HTTP wire: {request}"
    );

    settings::reset_for_test();
}

#[tokio::test]
#[serial]
async fn provider_4xx_with_tools_returns_interim_fallback() {
    let addr = spawn_loopback_openai_compat_tool_reject_fixture().await;
    init_http_config(&format!("http://{addr}/v1"), NativeToolsMode::True);

    let response = broker_inference::run_broker_inference(&BrokerInferenceRequest {
        prompt: String::new(),
        mode: "ask".to_string(),
        model: String::new(),
        messages: vec![rex_proto::rex::v1::ChatMessage {
            role: "user".to_string(),
            content: "hello".to_string(),
        }],
        tools: vec![ToolSpec {
            name: "fs.read".to_string(),
            description: String::new(),
            parameters_json: "{}".to_string(),
        }],
    })
    .await
    .expect("broker inference");

    assert!(!response.ok);
    assert_eq!(
        response.protocol,
        InferenceProtocol::InterimFallback as i32
    );
    assert!(
        response.error.contains("native_tools_unsupported"),
        "expected native_tools_unsupported, got: {}",
        response.error
    );

    settings::reset_for_test();
}

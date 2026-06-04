//! Cross-boundary conformance for `rex-cli` NDJSON and the editor consumer ([docs/EXTENSION.md](../../../docs/EXTENSION.md)).

use serde_json::Value;

const HAPPY_PATH_FIXTURE: &str =
    include_str!("../../../fixtures/ndjson_contract/happy_path.ndjson");
const TOOL_STEP_FIXTURE: &str =
    include_str!("../../../fixtures/ndjson_contract/tool_step_stream.ndjson");
const SETUP_ERRORS_FIXTURE: &str =
    include_str!("../../../fixtures/ndjson_contract/sidecar_setup_errors.ndjson");

fn terminal_event_count(lines: &str) -> usize {
    lines
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|value| {
            value
                .get("event")
                .and_then(|event| event.as_str())
                .is_some_and(|name| name == "done" || name == "error")
        })
        .count()
}

#[test]
fn shared_fixture_has_single_terminal_event() {
    assert_eq!(
        terminal_event_count(HAPPY_PATH_FIXTURE),
        1,
        "EXTENSION.md requires exactly one terminal event per successful stream"
    );
}

#[test]
fn shared_fixture_lines_match_stream_contract_fields() {
    for line in HAPPY_PATH_FIXTURE.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(trimmed).expect("each line must be JSON");
        let event = value
            .get("event")
            .and_then(|e| e.as_str())
            .expect("event field required");
        match event {
            "chunk" => {
                value
                    .get("index")
                    .and_then(|v| v.as_u64())
                    .expect("chunk.index");
                value
                    .get("text")
                    .and_then(|v| v.as_str())
                    .expect("chunk.text");
            }
            "done" => {
                value
                    .get("index")
                    .and_then(|v| v.as_u64())
                    .expect("done.index");
            }
            "error" => {
                value
                    .get("message")
                    .and_then(|v| v.as_str())
                    .expect("error.message");
            }
            "tool" => {
                value
                    .get("index")
                    .and_then(|v| v.as_u64())
                    .expect("tool.index");
                value
                    .get("name")
                    .and_then(|v| v.as_str())
                    .expect("tool.name");
                value
                    .get("phase")
                    .and_then(|v| v.as_str())
                    .expect("tool.phase");
            }
            "step" => {
                value
                    .get("index")
                    .and_then(|v| v.as_u64())
                    .expect("step.index");
                value
                    .get("phase")
                    .and_then(|v| v.as_str())
                    .expect("step.phase");
                value
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .expect("step.summary");
            }
            other => panic!("unexpected event: {other}"),
        }
    }
}

#[test]
fn tool_step_fixture_has_single_terminal_event() {
    assert_eq!(terminal_event_count(TOOL_STEP_FIXTURE), 1);
}

#[test]
fn tool_step_fixture_lines_match_stream_contract_fields() {
    for line in TOOL_STEP_FIXTURE.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(trimmed).expect("each line must be JSON");
        let event = value
            .get("event")
            .and_then(|e| e.as_str())
            .expect("event field required");
        match event {
            "chunk" => {
                value
                    .get("index")
                    .and_then(|v| v.as_u64())
                    .expect("chunk.index");
                value
                    .get("text")
                    .and_then(|v| v.as_str())
                    .expect("chunk.text");
            }
            "tool" => {
                value
                    .get("index")
                    .and_then(|v| v.as_u64())
                    .expect("tool.index");
                value
                    .get("name")
                    .and_then(|v| v.as_str())
                    .expect("tool.name");
                value
                    .get("phase")
                    .and_then(|v| v.as_str())
                    .expect("tool.phase");
            }
            "step" => {
                value
                    .get("index")
                    .and_then(|v| v.as_u64())
                    .expect("step.index");
                value
                    .get("phase")
                    .and_then(|v| v.as_str())
                    .expect("step.phase");
                value
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .expect("step.summary");
            }
            "done" => {
                value
                    .get("index")
                    .and_then(|v| v.as_u64())
                    .expect("done.index");
            }
            other => panic!("unexpected event: {other}"),
        }
    }
}

#[test]
fn setup_error_fixture_codes_are_stable() {
    let lines: Vec<&str> = SETUP_ERRORS_FIXTURE
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();
    assert_eq!(lines.len(), 2);
    let first: Value = serde_json::from_str(lines[0]).expect("json");
    let second: Value = serde_json::from_str(lines[1]).expect("json");
    assert_eq!(
        first.get("code").and_then(|v| v.as_str()),
        Some("sidecar_unavailable")
    );
    assert_eq!(
        second.get("code").and_then(|v| v.as_str()),
        Some("inference_config")
    );
}

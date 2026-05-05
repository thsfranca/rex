//! Cross-boundary conformance for `rex-cli` NDJSON and the editor consumer ([docs/EXTENSION.md](../../../docs/EXTENSION.md)).

use serde_json::Value;

const HAPPY_PATH_FIXTURE: &str =
    include_str!("../../../fixtures/ndjson_contract/happy_path.ndjson");

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
            other => panic!("unexpected event: {other}"),
        }
    }
}

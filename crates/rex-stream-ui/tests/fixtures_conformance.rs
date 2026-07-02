//! Fixture-driven conformance tests for stream consumer.

use rex_stream_ui::{StreamConsumer, TurnPhase, UiEffect};

const FIXTURES: &[(&str, &str)] = &[
    ("happy_path", include_str!("../../../fixtures/ndjson_contract/happy_path.ndjson")),
    (
        "tool_step_stream",
        include_str!("../../../fixtures/ndjson_contract/tool_step_stream.ndjson"),
    ),
    (
        "activity_stream",
        include_str!("../../../fixtures/ndjson_contract/activity_stream.ndjson"),
    ),
    (
        "plan_stream",
        include_str!("../../../fixtures/ndjson_contract/plan_stream.ndjson"),
    ),
];

fn terminal_count(effects: &[UiEffect]) -> usize {
    effects
        .iter()
        .filter(|e| matches!(e, UiEffect::TerminalDone | UiEffect::TerminalError { .. }))
        .count()
}

#[test]
fn all_success_fixtures_reach_terminal_done() {
    for (name, fixture) in FIXTURES {
        let mut consumer = StreamConsumer::new();
        let mut terminals = 0usize;
        for line in fixture.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let effects = consumer
                .feed_line(line)
                .unwrap_or_else(|err| panic!("{name}: {err}"));
            terminals += terminal_count(&effects);
        }
        assert_eq!(
            terminals, 1,
            "fixture {name} should produce exactly one terminal done"
        );
        assert_eq!(consumer.state.phase, TurnPhase::Idle);
    }
}

#[test]
fn tool_step_fixture_tracks_tool_by_call_id() {
    let fixture = include_str!("../../../fixtures/ndjson_contract/tool_step_stream.ndjson");
    let mut consumer = StreamConsumer::new();
    for line in fixture.lines() {
        if line.trim().is_empty() {
            continue;
        }
        consumer.feed_line(line).unwrap();
    }
    assert!(!consumer.state.active_tools.is_empty());
    assert!(consumer
        .state
        .active_tools
        .values()
        .any(|t| t.name == "fs.read" && t.completed));
}

#[test]
fn activity_stream_emits_operator_messages() {
    let fixture = include_str!("../../../fixtures/ndjson_contract/activity_stream.ndjson");
    let mut consumer = StreamConsumer::new();
    let mut messages = 0usize;
    for line in fixture.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let effects = consumer.feed_line(line).unwrap();
        messages += effects
            .iter()
            .filter(|e| matches!(e, UiEffect::OperatorMessage(_)))
            .count();
    }
    assert!(messages >= 2, "activity fixture should emit operator messages");
}

#[test]
fn approval_required_fixture_enters_tool_approval_phase() {
    let fixture =
        include_str!("../../../fixtures/ndjson_contract/tool_approval_required.ndjson");
    let mut consumer = StreamConsumer::new();
    let mut saw_approval = false;
    for line in fixture.lines() {
        if line.trim().is_empty() {
            continue;
        }
        consumer.feed_line(line).unwrap();
        if consumer.state.phase == TurnPhase::ToolApproval {
            saw_approval = true;
        }
    }
    assert!(saw_approval, "fixture should enter ToolApproval");
}

#[test]
fn error_fixture_produces_terminal_error() {
    let fixture =
        include_str!("../../../fixtures/ndjson_contract/sidecar_setup_errors.ndjson");
    for line in fixture.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let mut consumer = StreamConsumer::new();
        let effects = consumer.feed_line(line).unwrap();
        assert!(
            effects
                .iter()
                .any(|e| matches!(e, UiEffect::TerminalError { .. })),
            "each error line should terminal"
        );
    }
}

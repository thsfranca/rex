"""Tool protocol parsing tests."""

from rex_agent.tools import (
    BATCH_MIXED_ERROR,
    TOOL_LIST,
    TOOL_PLAN_SAVE,
    TOOL_READ,
    TOOL_WEB_SEARCH,
    TOOL_WRITE,
    ToolCall,
    ToolGateContext,
    explicit_web_intent,
    normalize_plan_save_path,
    normalize_tool_batch,
    parse_model_output,
    should_bill_tool_step,
    tool_specs_for_subagent,
    tools_for_mode,
)


def test_plan_mode_allows_read_and_list() -> None:
    allowed = tools_for_mode("plan")
    assert TOOL_READ in allowed
    assert TOOL_LIST in allowed
    assert TOOL_PLAN_SAVE in allowed


def test_normalize_plan_save_path() -> None:
    assert normalize_plan_save_path("feature.md") == ".rex/plans/feature.md"
    assert normalize_plan_save_path(".rex/plans/x.md") == ".rex/plans/x.md"


def test_plan_clarify_json_parsed() -> None:
    raw = '{"type":"clarify","questions":[{"id":"q1","prompt":"Scope?"}]}'
    parsed = parse_model_output(raw, "plan")
    assert parsed.kind == "clarify"
    assert parsed.clarify_questions is not None
    assert parsed.clarify_questions[0]["prompt"] == "Scope?"


def test_plan_final_json_parsed() -> None:
    raw = '{"type":"final","plan":{"title":"T","steps":[]}}'
    parsed = parse_model_output(raw, "plan")
    assert parsed.kind == "final"
    assert parsed.plan is not None
    assert parsed.answer == "T"


def test_ask_mode_parses_plain_text_as_final() -> None:
    parsed = parse_model_output("hello stub", "ask")
    assert parsed.kind == "final"
    assert parsed.answer == "hello stub"


def test_tool_json_parsed() -> None:
    raw = '{"type":"tool","tool":"fs.read","args":{"path":"README.md"}}'
    parsed = parse_model_output(raw, "plan")
    assert parsed.kind == "tool"
    assert parsed.tool_call is not None
    assert parsed.tool_call.tool == "fs.read"


def test_plan_mode_rejects_write_tool() -> None:
    raw = '{"type":"tool","tool":"fs.write","args":{"path":"a.txt","content":"x"}}'
    parsed = parse_model_output(raw, "plan")
    assert parsed.kind == "error"


def test_tool_specs_for_subagent_plan_orchestrator() -> None:
    try:
        from rex.v1 import rex_pb2  # noqa: F401
    except ImportError:
        return

    specs = tool_specs_for_subagent("orchestrator", "plan")
    names = {spec.name for spec in specs}
    assert TOOL_READ in names
    assert TOOL_LIST in names
    assert TOOL_PLAN_SAVE in names
    for spec in specs:
        assert spec.parameters_json.startswith("{")


def test_tool_specs_for_subagent_viewer_masks_write() -> None:
    try:
        from rex.v1 import rex_pb2  # noqa: F401
    except ImportError:
        return

    specs = tool_specs_for_subagent("viewer", "agent")
    names = {spec.name for spec in specs}
    assert TOOL_READ in names
    assert TOOL_LIST in names
    assert "fs.write" not in names
    assert "exec.shell" not in names


def test_normalize_tool_batch_accepts_parallel_reads() -> None:
    calls = [
        ToolCall(tool=TOOL_READ, args={"path": "a.md"}),
        ToolCall(tool=TOOL_READ, args={"path": "b.md"}),
        ToolCall(tool=TOOL_READ, args={"path": "c.md"}),
    ]
    normalized, error, truncated = normalize_tool_batch(
        calls, mode="plan", subagent="viewer"
    )
    assert error is None
    assert normalized is not None
    assert len(normalized) == 3
    assert truncated is False


def test_normalize_tool_batch_rejects_mixed_write() -> None:
    calls = [
        ToolCall(tool=TOOL_READ, args={"path": "a.md"}),
        ToolCall(tool=TOOL_WRITE, args={"path": "a.md", "content": "x"}),
    ]
    normalized, error, truncated = normalize_tool_batch(
        calls, mode="agent", subagent="viewer"
    )
    assert normalized is None
    assert error == BATCH_MIXED_ERROR
    assert truncated is False


def test_normalize_tool_batch_single_plan_save() -> None:
    calls = [ToolCall(tool=TOOL_PLAN_SAVE, args={"path": "p.md", "content": "# P"})]
    normalized, error, truncated = normalize_tool_batch(
        calls, mode="plan", subagent="orchestrator"
    )
    assert error is None
    assert normalized == calls
    assert truncated is False


def test_should_bill_tool_step_ok_result() -> None:
    assert should_bill_tool_step([(True, "ok body")]) is True


def test_should_bill_tool_step_policy_deny_not_billed() -> None:
    assert (
        should_bill_tool_step(
            [(False, "access policy denied (mode_denied): web.search denied for mode ask")]
        )
        is False
    )


def test_should_bill_tool_step_exploratory_failure_billed() -> None:
    assert should_bill_tool_step([(False, "file not found: missing.md")]) is True


def test_ask_tools_omit_web_search_when_search_disabled() -> None:
    gate = ToolGateContext(search_enabled=False, workspace_explored=True)
    allowed = tools_for_mode("ask", gate=gate)
    assert TOOL_WEB_SEARCH not in allowed
    assert TOOL_READ in allowed


def test_ask_tools_omit_web_search_until_workspace_explored() -> None:
    gate = ToolGateContext(search_enabled=True, workspace_explored=False)
    assert TOOL_WEB_SEARCH not in tools_for_mode("ask", gate=gate)


def test_ask_tools_allow_web_search_after_workspace_explored() -> None:
    gate = ToolGateContext(search_enabled=True, workspace_explored=True)
    assert TOOL_WEB_SEARCH in tools_for_mode("ask", gate=gate)


def test_explicit_web_intent_detects_online_lookup() -> None:
    assert explicit_web_intent("Please search the web for Rex framework") is True
    assert explicit_web_intent("What is rex?") is False


def test_ask_normalize_rejects_web_search_before_workspace() -> None:
    calls = [ToolCall(tool=TOOL_WEB_SEARCH, args={"query": "rex"})]
    gate = ToolGateContext(search_enabled=True, workspace_explored=False)
    normalized, error, truncated = normalize_tool_batch(
        calls, mode="ask", subagent="viewer", gate=gate
    )
    assert normalized is None
    assert error is not None
    assert "workspace" in error.lower()
    assert truncated is False


def test_ask_normalize_rejects_mixed_read_and_search() -> None:
    calls = [
        ToolCall(tool=TOOL_READ, args={"path": "README.md"}),
        ToolCall(tool=TOOL_WEB_SEARCH, args={"query": "rex"}),
    ]
    gate = ToolGateContext(search_enabled=True, workspace_explored=True)
    normalized, error, truncated = normalize_tool_batch(
        calls, mode="ask", subagent="viewer", gate=gate
    )
    assert normalized is None
    assert error is not None
    assert "web.search" in error
    assert truncated is False


def test_ask_tool_specs_omit_web_search_when_disabled() -> None:
    try:
        from rex.v1 import rex_pb2  # noqa: F401
    except ImportError:
        return

    gate = ToolGateContext(search_enabled=False)
    specs = tool_specs_for_subagent("viewer", "ask", gate=gate)
    names = {spec.name for spec in specs}
    assert TOOL_WEB_SEARCH not in names
    assert TOOL_READ in names

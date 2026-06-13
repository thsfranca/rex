"""Tool protocol parsing tests."""

from __future__ import annotations

from unittest.mock import MagicMock, patch

from rex_agent.graph.nodes.init import (
    prompt_has_explicit_file_reference,
    should_run_deterministic_init,
)
from rex_agent.graph.nodes.tools import AGENT_LOOP_STUCK_CODE
from rex_agent.tools import (
    BATCH_MIXED_ERROR,
    TOOL_LIST,
    TOOL_PLAN_SAVE,
    TOOL_READ,
    TOOL_WEB_SEARCH,
    TOOL_WRITE,
    ToolCall,
    ToolGateContext,
    ToolResultCache,
    explicit_web_intent,
    execute_tool,
    is_policy_config_failure,
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


def test_normalize_tool_batch_rejects_empty_read_path() -> None:
    calls = [ToolCall(tool=TOOL_READ, args={})]
    normalized, error, truncated = normalize_tool_batch(
        calls, mode="ask", subagent="viewer"
    )
    assert normalized is None
    assert error == "fs.read requires a non-empty path argument."
    assert truncated is False


def test_coerce_tool_args_maps_file_alias_to_path() -> None:
    from rex_agent.tools import coerce_tool_args, normalize_tool_call

    normalized, error = normalize_tool_call(
        ToolCall(tool=TOOL_READ, args={"file": "README.md"})
    )
    assert error is None
    assert normalized is not None
    assert normalized.args["path"] == "README.md"
    assert coerce_tool_args(TOOL_READ, {"file": "docs/X.md"})["path"] == "docs/X.md"


class _TreeBrokerClient:
    def __init__(
        self,
        *,
        files: set[str],
        dirs: dict[str, list[tuple[str, bool]]],
    ) -> None:
        self.files = files
        self.dirs = dirs
        self.read_calls: list[str] = []

    def read_file(self, path: str, mode: str) -> tuple[bool, str]:
        self.read_calls.append(path)
        if path in self.files:
            return True, f"<<TOOL_RESULT:fs.read>>\nbody:{path}\n<<END>>"
        return False, f"path not found: {path}"

    def list_dir_entries(
        self, path: str, mode: str | None = None
    ) -> tuple[bool, list[tuple[str, bool]] | str]:
        return True, self.dirs.get(path, [])


def test_find_paths_by_basename_discovers_unique_match() -> None:
    from rex_agent.tools import find_paths_by_basename

    client = _TreeBrokerClient(
        files={"docs/architecture/decisions/0001-example.md"},
        dirs={
            "": [("docs", True)],
            "docs": [("architecture", True)],
            "docs/architecture": [("decisions", True)],
            "docs/architecture/decisions": [("0001-example.md", False)],
        },
    )
    matches = find_paths_by_basename(client, "0001-example.md", "ask")
    assert matches == ["docs/architecture/decisions/0001-example.md"]


def test_execute_tool_read_resolves_unique_basename_match() -> None:
    from rex_agent.tools import ToolCall, execute_tool

    client = _TreeBrokerClient(
        files={"docs/architecture/decisions/0001-example.md"},
        dirs={
            "": [("docs", True)],
            "docs": [("architecture", True)],
            "docs/architecture": [("decisions", True)],
            "docs/architecture/decisions": [("0001-example.md", False)],
        },
    )
    call = ToolCall(
        tool=TOOL_READ,
        args={
            "path": (
                "docs/architecturedecisions/"
                "0001-example.md"
            )
        },
    )
    ok, result, truncated, cached = execute_tool(client, call, "ask")
    assert ok is True
    assert "resolved" in result
    assert "docs/architecture/decisions/0001-example.md" in result
    assert "body:docs/architecture/decisions/0001-example.md" in result
    assert client.read_calls[0].endswith("0001-example.md")
    assert client.read_calls[-1] == "docs/architecture/decisions/0001-example.md"
    assert truncated is False
    assert cached is False


def test_execute_tool_read_lists_ambiguous_basename_matches() -> None:
    from rex_agent.tools import ToolCall, execute_tool

    client = _TreeBrokerClient(
        files={"docs/a/README.md", "docs/b/README.md"},
        dirs={
            "": [("docs", True)],
            "docs": [("a", True), ("b", True)],
            "docs/a": [("README.md", False)],
            "docs/b": [("README.md", False)],
        },
    )
    call = ToolCall(tool=TOOL_READ, args={"path": "README.md"})
    ok, result, truncated, cached = execute_tool(client, call, "ask")
    assert ok is False
    assert "Basename 'README.md' exists at:" in result
    assert "docs/a/README.md" in result
    assert "docs/b/README.md" in result
    assert truncated is False
    assert cached is False


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
    deny_msg = "access policy denied (mode_denied): web.search denied for mode ask"
    assert should_bill_tool_step([(False, deny_msg)]) is False


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


def test_should_run_deterministic_init_ask_only() -> None:
    state = {
        "mode": "ask",
        "workspace_explored": False,
        "goal_hint": "What is rex?",
        "daemon_context": "user question",
    }
    with patch(
        "rex_agent.graph.nodes.init.deterministic_init_enabled", return_value=True
    ):
        assert should_run_deterministic_init(state) is True

    state["mode"] = "plan"
    assert should_run_deterministic_init(state) is False


def test_should_skip_deterministic_init_for_explicit_path() -> None:
    assert prompt_has_explicit_file_reference("Explain src/main.rs please")
    state = {
        "mode": "ask",
        "workspace_explored": False,
        "goal_hint": "Explain src/main.rs please",
        "daemon_context": "",
    }
    with patch(
        "rex_agent.graph.nodes.init.deterministic_init_enabled", return_value=True
    ):
        assert should_run_deterministic_init(state) is False


def test_policy_deny_batch_increments_error_count_semantics() -> None:
    deny = "access policy denied (mode_denied): web.search denied"
    assert is_policy_config_failure(deny) is True
    assert should_bill_tool_step([(False, deny)]) is False


def test_agent_loop_stuck_code_constant() -> None:
    assert AGENT_LOOP_STUCK_CODE == "agent_loop_stuck"


def test_exact_match_cache_bills_once_for_three_reads() -> None:
    client = MagicMock()
    client.read_file.return_value = (True, "<<TOOL_RESULT:fs.read>>\nbody\n<<END>>")
    cache = ToolResultCache()
    call = ToolCall(tool=TOOL_READ, args={"path": "main.rs"})
    ok1, _, _, dup1 = execute_tool(client, call, "agent", read_cache=cache)
    ok2, _, _, dup2 = execute_tool(client, call, "agent", read_cache=cache)
    ok3, _, _, dup3 = execute_tool(client, call, "agent", read_cache=cache)
    assert ok1 and not dup1
    assert ok2 and dup2
    assert ok3 and dup3
    client.read_file.assert_called_once()

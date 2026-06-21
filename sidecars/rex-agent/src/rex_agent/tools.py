"""Tool registry, ReAct JSON protocol, and broker execution."""

from __future__ import annotations

import json
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from rex_agent.broker import BrokerClient, strip_tool_result_delimiters
from rex_agent.config import (
    max_tool_result_bytes,
    max_tools_per_step,
    read_pruning_enabled,
)
from rex_agent.diff import (
    apply_unified_diff,
    editor_write_prompt_suffix,
    reject_whole_file_write,
)

TOOL_READ = "fs.read"
TOOL_LIST = "fs.list"
TOOL_WRITE = "fs.write"
TOOL_EXEC = "exec.shell"
TOOL_PLAN_SAVE = "plan.save"
TOOL_WEB_SEARCH = "web.search"
TOOL_WORKSPACE_SEARCH = "workspace.search"


def injected_files_system_note(injected_files: list[str]) -> str:
    paths = sorted(
        {
            path.strip().lstrip("./").lower()
            for path in injected_files
            if path.strip()
        }
    )
    if not paths:
        return ""
    joined = ", ".join(paths)
    return (
        "The daemon already embedded file contents in context for: "
        f"{joined}. Do not call fs.read on these paths unless the user "
        "asks for updated content."
    )


MAX_CLARIFY_QUESTIONS = 3
PLAN_PROMPT_SLICE = (
    "Plan mode: explore with fs.read/fs.list only; no patches or shell. "
    "Batch reads when helpful. "
    'Use {"type":"clarify","questions":[...]} (max 3) when scope is unclear. '
    'Finish with {"type":"final","plan":{title,steps[],risks[],open_questions[]}} '
    "or plan.save to .rex/plans/<name>.md when the user should persist the plan."
)
ASK_PROMPT_SLICE = (
    "Ask mode: read-only research. If the user asks a question that does not require "
    "file changes, answer from injected context first. Use fs.read, fs.list, or "
    "workspace.search only when context is insufficient or the user names paths. "
    "Use web.search only after local docs are insufficient or the user explicitly "
    "asked for web lookup. Minimize tool rounds; cite sources in your final answer."
)
AGENT_PROMPT_SLICE = (
    "Agent mode: batch viewer reads before editing. Use exactly one fs.write or "
    "exec.shell per editor step."
)

TOOLS_BY_MODE: dict[str, frozenset[str]] = {
    "ask": frozenset({TOOL_READ, TOOL_LIST, TOOL_WEB_SEARCH, TOOL_WORKSPACE_SEARCH}),
    "plan": frozenset({TOOL_READ, TOOL_LIST, TOOL_PLAN_SAVE, TOOL_WORKSPACE_SEARCH}),
    "agent": frozenset(
        {TOOL_READ, TOOL_LIST, TOOL_WRITE, TOOL_EXEC, TOOL_WORKSPACE_SEARCH}
    ),
}

VIEWER_TOOLS = frozenset(
    {TOOL_READ, TOOL_LIST, TOOL_WEB_SEARCH, TOOL_WORKSPACE_SEARCH}
)
EDITOR_TOOLS = frozenset({TOOL_READ, TOOL_WRITE, TOOL_EXEC})

BATCHABLE_READ_TOOLS = frozenset({TOOL_READ, TOOL_LIST, TOOL_WORKSPACE_SEARCH})

FIND_MAX_DEPTH = 12
FIND_MAX_DIRS = 96
FIND_MAX_MATCHES = 8
FIND_SKIP_DIR_NAMES = frozenset(
    {".git", "node_modules", "target", "__pycache__", ".venv", "venv", "dist", "build"}
)
NON_BATCHABLE_TOOLS = frozenset({TOOL_WRITE, TOOL_EXEC, TOOL_PLAN_SAVE})

BATCH_MIXED_ERROR = (
    "Cannot mix read-only batch tools with write, exec, or plan.save in one step. "
    "Request read/list/search tools together, or a single write/exec/plan.save alone."
)
BATCH_EDITOR_MULTI_ERROR = (
    "Editor step allows exactly one fs.write or exec.shell tool per step."
)
BATCH_TRUNCATED_NOTE = (
    "Tool batch exceeded agent.max_tools_per_step; extra tool calls were not executed."
)

ASK_WEB_SEARCH_WORKSPACE_FIRST = (
    "web.search is not available until you have read or listed workspace files. "
    "Use fs.read or fs.list first, or ask the user to request a web search explicitly."
)
ASK_WEB_SEARCH_MIXED_BATCH = (
    "Cannot batch web.search with fs.read or fs.list in ask mode. "
    "Use workspace tools first, then web.search in a separate step."
)

_EXPLICIT_WEB_INTENT = re.compile(
    r"\b(search the web|web search|look online|google|internet|latest news|online)\b",
    re.IGNORECASE,
)

_POLICY_CONFIG_FAILURE_MARKERS = (
    "mode_denied",
    "access policy denied",
    "plan_save_denied",
    "requires path",
    "requires query",
    "requires content",
    "not allowed in",
    "unknown tool:",
    "web.search is not available until",
    "cannot batch web.search",
)

TOOL_DESCRIPTIONS: dict[str, str] = {
    TOOL_READ: "Read a file relative to the workspace root",
    TOOL_LIST: "List directory entries relative to the workspace root",
    TOOL_WRITE: "Write or patch a file (content or unified diff)",
    TOOL_EXEC: "Run an allowlisted shell command in the workspace",
    TOOL_PLAN_SAVE: "Save a markdown plan under .rex/plans/",
    TOOL_WEB_SEARCH: "Search the web for up-to-date information",
    TOOL_WORKSPACE_SEARCH: "Search workspace paths by basename or file content",
}

TOOL_SCHEMAS: dict[str, dict[str, Any]] = {
    TOOL_READ: {
        "type": "object",
        "properties": {
            "path": {"type": "string", "description": "Relative file path"},
        },
        "required": ["path"],
    },
    TOOL_LIST: {
        "type": "object",
        "properties": {
            "path": {
                "type": "string",
                "description": "Relative directory path (empty for workspace root)",
            },
        },
    },
    TOOL_WRITE: {
        "type": "object",
        "properties": {
            "path": {"type": "string", "description": "Relative file path"},
            "content": {"type": "string", "description": "Full file content"},
            "diff": {"type": "string", "description": "Unified diff to apply"},
        },
        "required": ["path"],
    },
    TOOL_EXEC: {
        "type": "object",
        "properties": {
            "command": {"type": "string", "description": "Shell command to run"},
        },
        "required": ["command"],
    },
    TOOL_PLAN_SAVE: {
        "type": "object",
        "properties": {
            "path": {
                "type": "string",
                "description": "Plan filename (saved under .rex/plans/)",
            },
            "content": {"type": "string", "description": "Markdown plan body"},
        },
        "required": ["path", "content"],
    },
    TOOL_WEB_SEARCH: {
        "type": "object",
        "properties": {
            "query": {"type": "string", "description": "Search query"},
        },
        "required": ["query"],
    },
    TOOL_WORKSPACE_SEARCH: {
        "type": "object",
        "properties": {
            "query": {"type": "string", "description": "Search terms"},
            "kind": {
                "type": "string",
                "description": "basename or content",
                "enum": ["basename", "content"],
            },
        },
        "required": ["query"],
    },
}

try:
    from rex.v1 import rex_pb2
except ImportError:  # pragma: no cover
    rex_pb2 = None  # type: ignore[assignment]


@dataclass(frozen=True)
class ToolCall:
    tool: str
    args: dict[str, Any]


def _non_empty_str(value: Any) -> str:
    if not isinstance(value, str):
        return ""
    return value.strip()


def _join_relative_path(base: str, name: str) -> str:
    if not base:
        return name
    return f"{base}/{name}"


def is_read_path_not_found(error: str) -> bool:
    lower = error.lower().strip()
    return lower.startswith("path not found:") or lower.startswith("not found:")


def find_paths_by_basename(
    client: BrokerClient,
    basename: str,
    mode: str,
) -> list[str]:
    """Breadth-first workspace search for relative paths matching basename."""
    if not basename or basename in (".", ".."):
        return []
    matches: list[str] = []
    queue: list[tuple[str, int]] = [("", 0)]
    visited = 0
    while queue and visited < FIND_MAX_DIRS and len(matches) < FIND_MAX_MATCHES:
        dir_path, depth = queue.pop(0)
        visited += 1
        ok, payload = client.list_dir_entries(dir_path, mode)
        if not ok or not isinstance(payload, list):
            continue
        for name, is_dir in payload:
            if is_dir:
                if name in FIND_SKIP_DIR_NAMES or depth >= FIND_MAX_DEPTH:
                    continue
                queue.append((_join_relative_path(dir_path, name), depth + 1))
                continue
            if name == basename:
                matches.append(_join_relative_path(dir_path, name))
                if len(matches) >= FIND_MAX_MATCHES:
                    return matches
    return matches


def resolve_read_path_after_not_found(
    client: BrokerClient,
    requested_path: str,
    mode: str,
) -> tuple[str | None, str | None]:
    """Search by basename when a read path is missing."""
    basename = Path(requested_path).name
    if not basename:
        return None, None
    matches = find_paths_by_basename(client, basename, mode)
    if len(matches) == 1:
        return matches[0], None
    if not matches:
        return None, None
    listed = ", ".join(matches)
    overflow = ""
    if len(matches) >= FIND_MAX_MATCHES:
        overflow = f" (showing first {FIND_MAX_MATCHES} matches)"
    return None, (
        f"path not found: {requested_path}. "
        f"Basename {basename!r} exists at: {listed}{overflow}"
    )


def coerce_tool_args(tool: str, args: dict[str, Any] | None) -> dict[str, Any]:
    """Normalize common provider arg aliases before validation."""
    coerced = dict(args or {})
    if tool in (TOOL_READ, TOOL_WRITE, TOOL_LIST, TOOL_PLAN_SAVE):
        if not _non_empty_str(coerced.get("path")):
            for alt in ("file", "filepath", "file_path", "filename", "name"):
                candidate = _non_empty_str(coerced.get(alt))
                if candidate:
                    coerced["path"] = candidate
                    break
    if tool == TOOL_WEB_SEARCH and not _non_empty_str(coerced.get("query")):
        for alt in ("q", "search", "text", "terms"):
            candidate = _non_empty_str(coerced.get(alt))
            if candidate:
                coerced["query"] = candidate
                break
    if tool == TOOL_EXEC and not _non_empty_str(coerced.get("command")):
        for alt in ("cmd", "shell", "script"):
            candidate = _non_empty_str(coerced.get(alt))
            if candidate:
                coerced["command"] = candidate
                break
    return coerced


def validate_tool_call(call: ToolCall) -> str | None:
    """Return operator-facing error when required args are missing."""
    args = call.args
    if call.tool == TOOL_READ:
        if not _non_empty_str(args.get("path")):
            return "fs.read requires a non-empty path argument."
    elif call.tool == TOOL_WRITE:
        if not _non_empty_str(args.get("path")):
            return "fs.write requires a non-empty path argument."
    elif call.tool == TOOL_WEB_SEARCH:
        if not _non_empty_str(args.get("query")):
            return "web.search requires a non-empty query argument."
    elif call.tool == TOOL_WORKSPACE_SEARCH:
        if not _non_empty_str(args.get("query")):
            return "workspace.search requires a non-empty query argument."
    elif call.tool == TOOL_EXEC:
        if not _non_empty_str(args.get("command")):
            return "exec.shell requires a non-empty command argument."
    elif call.tool == TOOL_PLAN_SAVE:
        if not _non_empty_str(args.get("path")):
            return "plan.save requires a non-empty path argument."
        if not _non_empty_str(args.get("content")):
            return "plan.save requires content."
    return None


def normalize_tool_call(call: ToolCall) -> tuple[ToolCall | None, str | None]:
    coerced = coerce_tool_args(call.tool, call.args)
    normalized = ToolCall(tool=call.tool, args=coerced)
    error = validate_tool_call(normalized)
    if error:
        return None, error
    return normalized, None


@dataclass(frozen=True)
class ParsedModelOutput:
    kind: str
    answer: str = ""
    tool_call: ToolCall | None = None
    message: str = ""
    plan: dict[str, Any] | None = None
    clarify_questions: list[dict[str, Any]] | None = None


@dataclass(frozen=True)
class ToolGateContext:
    search_enabled: bool = False
    workspace_explored: bool = False
    explicit_web_intent: bool = False

    @classmethod
    def defaults(cls) -> ToolGateContext:
        from rex_agent.config import search_enabled

        return cls(search_enabled=search_enabled())

    @classmethod
    def from_goal_hint(
        cls,
        goal_hint: str,
        *,
        workspace_explored: bool = False,
    ) -> ToolGateContext:
        from rex_agent.config import search_enabled

        return cls(
            search_enabled=search_enabled(),
            workspace_explored=workspace_explored,
            explicit_web_intent=explicit_web_intent(goal_hint),
        )


@dataclass
class ToolResultCache:
    """Intra-turn exact-match cache keyed by tool name + canonical args JSON."""

    entries: dict[str, tuple[bool, str]] = field(default_factory=dict)

    @staticmethod
    def cache_key(tool: str, args: dict[str, Any]) -> str:
        payload = json.dumps(args, sort_keys=True, separators=(",", ":"))
        return f"{tool}:{payload}"

    def get_call(self, tool: str, args: dict[str, Any]) -> tuple[bool, str] | None:
        return self.entries.get(self.cache_key(tool, args))

    def put_call(self, tool: str, args: dict[str, Any], ok: bool, result: str) -> None:
        self.entries[self.cache_key(tool, args)] = (ok, result)


# Backward-compatible alias for state field name.
ReadCache = ToolResultCache


def explicit_web_intent(prompt: str) -> bool:
    return bool(_EXPLICIT_WEB_INTENT.search(prompt or ""))


def ask_web_search_allowed(gate: ToolGateContext) -> bool:
    return gate.search_enabled and (
        gate.workspace_explored or gate.explicit_web_intent
    )


def is_policy_config_failure(result: str) -> bool:
    lower = (result or "").lower()
    return any(marker in lower for marker in _POLICY_CONFIG_FAILURE_MARKERS)


def should_bill_tool_step(results: list[tuple[bool, str]]) -> bool:
    """Return True when a tool batch should count toward max_tool_steps*."""
    if not results:
        return False
    if any(ok for ok, _ in results):
        return True
    return not all(is_policy_config_failure(result) for _, result in results)


def tools_for_mode(
    mode: str,
    *,
    gate: ToolGateContext | None = None,
) -> frozenset[str]:
    normalized = (mode or "ask").strip().lower() or "ask"
    allowed = set(TOOLS_BY_MODE.get(normalized, TOOLS_BY_MODE["ask"]))
    ctx = gate or ToolGateContext.defaults()
    if normalized == "ask" and TOOL_WEB_SEARCH in allowed:
        if not ask_web_search_allowed(ctx):
            allowed.discard(TOOL_WEB_SEARCH)
    return frozenset(allowed)


def tool_gate_from_state(state: dict[str, Any]) -> ToolGateContext:
    return ToolGateContext.from_goal_hint(
        str(state.get("goal_hint") or ""),
        workspace_explored=bool(state.get("workspace_explored")),
    )


def tools_for_subagent(
    subagent: str,
    mode: str,
    *,
    gate: ToolGateContext | None = None,
) -> frozenset[str]:
    allowed = tools_for_mode(mode, gate=gate)
    if subagent == "viewer":
        return allowed & VIEWER_TOOLS
    if subagent == "editor":
        return allowed & EDITOR_TOOLS
    return allowed


def batchable_tools_for_mode(mode: str) -> frozenset[str]:
    return frozenset(BATCHABLE_READ_TOOLS)


def is_batchable_tool(tool: str, mode: str) -> bool:
    return tool in batchable_tools_for_mode(mode)


def normalize_tool_batch(
    calls: list[ToolCall],
    *,
    mode: str,
    subagent: str,
    max_batch: int | None = None,
    gate: ToolGateContext | None = None,
) -> tuple[list[ToolCall] | None, str | None, bool]:
    """Validate and cap a tool batch. Returns (calls, error, truncated)."""
    if not calls:
        return None, "No tool calls in model response.", False

    cap = max_batch if max_batch is not None else max_tools_per_step()
    truncated = False
    ctx = gate or ToolGateContext.defaults()
    normalized_mode = (mode or "ask").strip().lower() or "ask"

    if subagent == "editor":
        if len(calls) != 1 or calls[0].tool not in (TOOL_WRITE, TOOL_EXEC):
            return None, BATCH_EDITOR_MULTI_ERROR, False
        return calls, None, False

    if normalized_mode == "ask":
        has_search = any(call.tool == TOOL_WEB_SEARCH for call in calls)
        has_workspace = any(
            call.tool in (TOOL_READ, TOOL_LIST) for call in calls
        )
        if has_search and has_workspace:
            return None, ASK_WEB_SEARCH_MIXED_BATCH, False
        if has_search and not ask_web_search_allowed(ctx):
            return None, ASK_WEB_SEARCH_WORKSPACE_FIRST, False

    batchable = batchable_tools_for_mode(mode)
    allowed_mode = tools_for_mode(mode, gate=ctx)

    if len(calls) == 1 and calls[0].tool not in batchable:
        if calls[0].tool in allowed_mode:
            return calls, None, False
        return None, f"Tool {calls[0].tool!r} is not allowed in {mode} mode.", False

    for call in calls:
        if call.tool not in batchable:
            return None, BATCH_MIXED_ERROR, False
        if call.tool not in allowed_mode:
            return None, f"Tool {call.tool!r} is not allowed in {mode} mode.", False

    if len(calls) > cap:
        calls = calls[:cap]
        truncated = True

    normalized_calls: list[ToolCall] = []
    for call in calls:
        normalized, error = normalize_tool_call(call)
        if error is not None or normalized is None:
            return None, error, False
        normalized_calls.append(normalized)

    return normalized_calls, None, truncated


def tool_specs_for_subagent(
    subagent: str,
    mode: str,
    *,
    gate: ToolGateContext | None = None,
) -> list[Any]:
    """OpenAI-shaped ToolSpec protos for native broker tool calling (R038)."""
    if rex_pb2 is None:
        raise ImportError(
            "rex.v1 protobuf stubs not found. Run `rex proto install`."
        )
    allowed = tools_for_subagent(subagent, mode, gate=gate)
    specs: list[Any] = []
    for name in sorted(allowed):
        schema = TOOL_SCHEMAS.get(name)
        if schema is None:
            continue
        specs.append(
            rex_pb2.ToolSpec(
                name=name,
                description=TOOL_DESCRIPTIONS.get(name, name),
                parameters_json=json.dumps(schema),
            )
        )
    return specs


def system_prompt_for_tools(
    mode: str,
    *,
    subagent: str = "orchestrator",
    gate: ToolGateContext | None = None,
) -> str:
    allowed = tools_for_subagent(subagent, mode, gate=gate)
    if not allowed:
        return (
            "You are a helpful assistant. Respond with your final answer "
            "as plain text. Do not request tools."
        )
    tool_docs = []
    if TOOL_READ in allowed:
        tool_docs.append(f'- "{TOOL_READ}": args {{"path": "<relative path>"}}')
    if TOOL_LIST in allowed:
        tool_docs.append(
            f'- "{TOOL_LIST}": args {{"path": "<relative dir or empty for root>"}}'
        )
    if TOOL_WRITE in allowed:
        tool_docs.append(
            f'- "{TOOL_WRITE}": args {{"path": "<relative path>", '
            '"content": "<text>" OR "diff": "<unified diff>"}}'
        )
    if TOOL_EXEC in allowed:
        tool_docs.append(f'- "{TOOL_EXEC}": args {{"command": "<shell command>"}}')
    if TOOL_PLAN_SAVE in allowed:
        tool_docs.append(
            f'- "{TOOL_PLAN_SAVE}": args '
            '{"path": "<name>.md", "content": "<markdown>"} '
            "(under .rex/plans/)"
        )
    if TOOL_WEB_SEARCH in allowed:
        tool_docs.append(f'- "{TOOL_WEB_SEARCH}": args {{"query": "<search terms>"}}')
    if TOOL_WORKSPACE_SEARCH in allowed:
        tool_docs.append(
            f'- "{TOOL_WORKSPACE_SEARCH}": args {{"query": "<terms>", '
            '"kind": "basename|content"}}'
        )
    if subagent == "editor":
        tool_policy = (
            "You are a development agent. Use exactly one fs.write or exec.shell "
            "tool per step (no batching with reads).\n"
        )
    else:
        tool_policy = (
            "You are a development agent. You may request multiple read/list/search "
            "tools in one native tool_calls response; that counts as one step.\n"
        )
    base = (
        f"{tool_policy}"
        "When you need a tool (interim JSON path), respond with ONLY a JSON object "
        "on one line:\n"
        '{"type":"tool","tool":"<name>","args":{...}}\n'
        "When you are done, respond with ONLY:\n"
        '{"type":"final","answer":"<your reply>"}\n'
        "Allowed tools:\n"
        f"{chr(10).join(tool_docs)}\n"
        "If the user message already contains file contents, "
        "do not re-read those paths."
    )
    if subagent == "editor" and TOOL_WRITE in allowed:
        base += "\n" + editor_write_prompt_suffix()
    if (mode or "ask").strip().lower() == "plan" and subagent in (
        "orchestrator",
        "viewer",
    ):
        base += "\n" + PLAN_PROMPT_SLICE + _workspace_mode_prompt_overlay("plan")
    if (mode or "ask").strip().lower() == "ask" and subagent in (
        "orchestrator",
        "viewer",
    ):
        base += "\n" + ASK_PROMPT_SLICE + _workspace_mode_prompt_overlay("ask")
    if (mode or "ask").strip().lower() == "agent" and subagent in (
        "orchestrator",
        "viewer",
        "editor",
    ):
        base += "\n" + AGENT_PROMPT_SLICE + _workspace_mode_prompt_overlay("agent")
    return base


def _workspace_mode_prompt_overlay(mode: str) -> str:
    normalized = (mode or "ask").strip().lower() or "ask"
    rel = f"prompts/mode/{normalized}.md"
    for candidate in (f".rex/{rel}", rel):
        try:
            text = Path(candidate).read_text(encoding="utf-8").strip()
        except OSError:
            continue
        if text:
            return f"\n[project {normalized} mode instructions]\n{text}"
    return ""


def normalize_plan_save_path(path: str) -> str:
    trimmed = path.strip().lstrip("/")
    if trimmed.startswith(".rex/plans/"):
        return trimmed
    name = trimmed.removeprefix(".rex/plans/")
    return f".rex/plans/{name}"


def _extract_json_object(text: str) -> str | None:
    stripped = text.strip()
    if stripped.startswith("{") and stripped.endswith("}"):
        return stripped
    match = re.search(r"\{[^{}]*\}", stripped, re.DOTALL)
    if match:
        return match.group(0)
    return None


_PARSE_JSON_ERROR = (
    "Could not parse model output as JSON. "
    "Reply with a final answer or valid tool JSON."
)


def parse_model_output(
    text: str,
    mode: str,
    *,
    gate: ToolGateContext | None = None,
) -> ParsedModelOutput:
    allowed = tools_for_mode(mode, gate=gate)
    raw = text.strip()
    if not raw:
        return ParsedModelOutput(
            kind="error", message="Model returned an empty response."
        )

    blob = _extract_json_object(raw)
    if blob is None:
        if raw.startswith("{") and allowed:
            return ParsedModelOutput(
                kind="error",
                message=_PARSE_JSON_ERROR,
            )
        return ParsedModelOutput(kind="final", answer=raw)

    try:
        payload = json.loads(blob)
    except json.JSONDecodeError:
        return ParsedModelOutput(
            kind="error",
            message=_PARSE_JSON_ERROR,
        )

    kind = str(payload.get("type", "")).strip().lower()
    if kind == "clarify":
        if (mode or "").strip().lower() != "plan":
            return ParsedModelOutput(
                kind="error",
                message="Clarify JSON is only valid in plan mode.",
            )
        raw_questions = payload.get("questions")
        if not isinstance(raw_questions, list) or not raw_questions:
            return ParsedModelOutput(
                kind="error",
                message="Clarify JSON must include a non-empty questions array.",
            )
        questions: list[dict[str, Any]] = []
        for item in raw_questions[:MAX_CLARIFY_QUESTIONS]:
            if not isinstance(item, dict):
                continue
            qid = str(item.get("id", "")).strip() or f"q{len(questions) + 1}"
            prompt = str(item.get("prompt", "")).strip()
            if not prompt:
                continue
            entry: dict[str, Any] = {"id": qid, "prompt": prompt}
            options = item.get("options")
            if isinstance(options, list) and options:
                entry["options"] = [str(o) for o in options[:8]]
            questions.append(entry)
        if not questions:
            return ParsedModelOutput(
                kind="error", message="Clarify questions must include prompts."
            )
        return ParsedModelOutput(kind="clarify", clarify_questions=questions)

    if kind == "final":
        plan_obj = payload.get("plan")
        if isinstance(plan_obj, dict) and (mode or "").strip().lower() == "plan":
            title = str(plan_obj.get("title", "")).strip() or "Plan"
            return ParsedModelOutput(
                kind="final",
                answer=title,
                plan=plan_obj,
            )
        answer = str(payload.get("answer", "")).strip()
        if not answer:
            return ParsedModelOutput(
                kind="error",
                message="Final answer JSON must include answer or plan object.",
            )
        return ParsedModelOutput(kind="final", answer=answer)

    if kind == "tool":
        tool = str(payload.get("tool", "")).strip()
        args = payload.get("args")
        if not isinstance(args, dict):
            return ParsedModelOutput(
                kind="error", message="Tool call JSON must include an args object."
            )
        if tool not in allowed:
            return ParsedModelOutput(
                kind="error", message=f"Tool {tool!r} is not allowed in {mode} mode."
            )
        return ParsedModelOutput(kind="tool", tool_call=ToolCall(tool=tool, args=args))

    return ParsedModelOutput(
        kind="error",
        message='Model JSON must use type "final", "tool", or "clarify" (plan mode).',
    )


def prune_read_result(content: str, goal_hint: str) -> str:
    lines = content.splitlines()
    if len(lines) <= 100:
        return content
    hint_tokens = {t.lower() for t in re.findall(r"\w+", goal_hint) if len(t) > 2}
    if not hint_tokens:
        return (
            "\n".join(lines[:40])
            + f"\n… [{len(lines) - 50} lines pruned] …\n"
            + "\n".join(lines[-10:])
        )
    scored = [
        (sum(1 for t in hint_tokens if t in line.lower()), line) for line in lines
    ]
    scored = [(s, ln) for s, ln in scored if s]
    if not scored:
        return prune_read_result(content, "")
    scored.sort(key=lambda x: -x[0])
    selected = sorted(scored[:60], key=lambda x: lines.index(x[1]))
    return (
        f"[pruned read: {len(lines)} lines → {len(selected)} goal-relevant lines]\n"
        + "\n".join(line for _, line in selected)
    )


def execute_tool(
    client: BrokerClient,
    call: ToolCall,
    mode: str,
    *,
    read_cache: ToolResultCache | None = None,
    goal_hint: str = "",
) -> tuple[bool, str, bool, bool]:
    tool = call.tool
    args = call.args
    truncated = False

    if read_cache is not None:
        cached = read_cache.get_call(tool, args)
        if cached is not None:
            ok, result = cached
            return ok, f"[cached {tool} result]\n{result}", False, True

    if tool == TOOL_READ:
        path = str(args.get("path", "")).strip()
        if not path:
            return False, "fs.read requires path", False, False
        resolve_note = ""
        ok, result = client.read_file(path, mode)
        if not ok and is_read_path_not_found(result):
            resolved, hint = resolve_read_path_after_not_found(client, path, mode)
            if resolved:
                ok, result = client.read_file(resolved, mode)
                if ok:
                    resolve_note = f"[fs.read: resolved {path} -> {resolved}]\n"
            elif hint:
                result = hint
        if ok:
            raw_body = strip_tool_result_delimiters(result)
            pruned = raw_body
            if read_pruning_enabled() and goal_hint:
                pruned = prune_read_result(raw_body, goal_hint)
            if read_cache is not None:
                read_cache.put_call(tool, args, True, result)
            if len(raw_body.encode("utf-8")) >= max_tool_result_bytes():
                truncated = True
            if " [rex: tool output truncated]" in raw_body:
                truncated = True
            if pruned != raw_body:
                result = format_delimited_tool_result_for_prompt(TOOL_READ, pruned)
            if resolve_note:
                result = resolve_note + result
        return ok, result, truncated, False

    if tool == TOOL_LIST:
        path = str(args.get("path", "")).strip()
        ok, result = client.list_dir(path, mode)
        if ok and read_cache is not None:
            read_cache.put_call(tool, args, ok, result)
        return ok, result, False, False

    if tool == TOOL_WEB_SEARCH:
        query = str(args.get("query", "")).strip()
        if not query:
            return False, "web.search requires query", False, False
        ok, result = client.web_search(query, mode)
        if ok and read_cache is not None:
            read_cache.put_call(tool, args, ok, result)
        return ok, result, False, False

    if tool == TOOL_WORKSPACE_SEARCH:
        query = str(args.get("query", "")).strip()
        if not query:
            return False, "workspace.search requires query", False, False
        kind = str(args.get("kind", "basename")).strip().lower() or "basename"
        ok, result = client.workspace_search(query, kind, mode)
        if ok and read_cache is not None:
            read_cache.put_call(tool, args, ok, result)
        return ok, result, False, False

    if tool == TOOL_WRITE:
        path = str(args.get("path", "")).strip()
        if not path:
            return False, "fs.write requires path", False, False
        diff_text = args.get("diff")
        content = args.get("content")
        if diff_text is not None and str(diff_text).strip():
            ok_read, existing = client.read_file(path, mode)
            if not ok_read:
                existing = ""
            else:
                existing = strip_tool_result_delimiters(existing)
            ok_patch, patched = apply_unified_diff(existing, str(diff_text))
            if not ok_patch:
                return False, patched, False, False
            ok, msg = client.write_file(path, patched, mode)
            return ok, msg if ok else msg, False, False
        if content is None:
            return False, "fs.write requires content or diff", False, False
        content_str = str(content)
        ok_read, existing = client.read_file(path, mode)
        if not ok_read:
            existing = ""
        else:
            existing = strip_tool_result_delimiters(existing)
        reject = reject_whole_file_write(path, content_str, existing)
        if reject:
            return False, reject, False, False
        ok, msg = client.write_file(path, content_str, mode)
        return ok, msg if ok else msg, False, False

    if tool == TOOL_EXEC:
        command = str(args.get("command", "")).strip()
        if not command:
            return False, "exec.shell requires command", False, False
        ok, result = client.exec_shell(command, mode)
        return ok, result, False, False

    if tool == TOOL_PLAN_SAVE:
        path = normalize_plan_save_path(str(args.get("path", "")))
        content = str(args.get("content", ""))
        if not path:
            return False, "plan.save requires path", False, False
        if not content.strip():
            return False, "plan.save requires content", False, False
        ok, msg = client.save_plan(path, content, mode)
        return ok, msg if ok else msg, False, False

    return False, f"Unknown tool: {tool}", False, False


def format_delimited_tool_result_for_prompt(tool: str, body: str) -> str:
    """Re-wrap stripped body for LLM scratch when sidecar re-processed content."""
    return f"<<TOOL_RESULT:{tool}>>\n{body}\n<<END>>"


def format_tool_status(call: ToolCall, ok: bool, result: str) -> str:
    return f"\n[tool {call.tool} {'ok' if ok else 'error'}]\n{result}\n"

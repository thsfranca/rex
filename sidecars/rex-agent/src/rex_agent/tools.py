"""Tool registry, ReAct JSON protocol, and broker execution."""

from __future__ import annotations

import json
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from rex_agent.broker import BrokerClient, strip_tool_result_delimiters
from rex_agent.config import max_tool_result_bytes, read_pruning_enabled
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

MAX_CLARIFY_QUESTIONS = 3
PLAN_PROMPT_SLICE = (
    "Plan mode: explore with fs.read/fs.list only; no patches or shell. "
    'Use {"type":"clarify","questions":[...]} (max 3) when scope is unclear. '
    'Finish with {"type":"final","plan":{title,steps[],risks[],open_questions[]}} '
    "or plan.save to .rex/plans/<name>.md when the user should persist the plan."
)

TOOLS_BY_MODE: dict[str, frozenset[str]] = {
    "ask": frozenset(),
    "plan": frozenset({TOOL_READ, TOOL_LIST, TOOL_PLAN_SAVE}),
    "agent": frozenset({TOOL_READ, TOOL_LIST, TOOL_WRITE, TOOL_EXEC}),
}

VIEWER_TOOLS = frozenset({TOOL_READ, TOOL_LIST})
EDITOR_TOOLS = frozenset({TOOL_READ, TOOL_WRITE, TOOL_EXEC})


@dataclass(frozen=True)
class ToolCall:
    tool: str
    args: dict[str, Any]


@dataclass(frozen=True)
class ParsedModelOutput:
    kind: str
    answer: str = ""
    tool_call: ToolCall | None = None
    message: str = ""
    plan: dict[str, Any] | None = None
    clarify_questions: list[dict[str, Any]] | None = None


@dataclass
class ReadCache:
    entries: dict[str, str] = field(default_factory=dict)

    def get(self, path: str) -> str | None:
        return self.entries.get(path)

    def put(self, path: str, content: str) -> None:
        self.entries[path] = content


def tools_for_mode(mode: str) -> frozenset[str]:
    normalized = (mode or "ask").strip().lower() or "ask"
    return TOOLS_BY_MODE.get(normalized, TOOLS_BY_MODE["ask"])


def tools_for_subagent(subagent: str, mode: str) -> frozenset[str]:
    allowed = tools_for_mode(mode)
    if subagent == "viewer":
        return allowed & VIEWER_TOOLS
    if subagent == "editor":
        return allowed & EDITOR_TOOLS
    return allowed


def system_prompt_for_tools(mode: str, *, subagent: str = "orchestrator") -> str:
    allowed = tools_for_subagent(subagent, mode)
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
    base = (
        "You are a development agent. Use at most one tool per step.\n"
        "When you need a tool, respond with ONLY a JSON object on one line:\n"
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
        base += "\n" + PLAN_PROMPT_SLICE + _workspace_plan_prompt_overlay()
    return base


def _workspace_plan_prompt_overlay() -> str:
    for candidate in (".rex/prompts/mode/plan.md", "prompts/mode/plan.md"):
        try:
            text = Path(candidate).read_text(encoding="utf-8").strip()
        except OSError:
            continue
        if text:
            return f"\n[project plan instructions]\n{text}"
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


def parse_model_output(text: str, mode: str) -> ParsedModelOutput:
    allowed = tools_for_mode(mode)
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
    read_cache: ReadCache | None = None,
    goal_hint: str = "",
) -> tuple[bool, str, bool]:
    tool = call.tool
    args = call.args
    truncated = False

    if tool == TOOL_READ:
        path = str(args.get("path", "")).strip()
        if not path:
            return False, "fs.read requires path", False
        if read_cache is not None:
            cached = read_cache.get(path)
            if cached is not None:
                return True, f"[cached read of {path}]\n{cached}", False
        ok, result = client.read_file(path, mode)
        if ok:
            raw_body = strip_tool_result_delimiters(result)
            pruned = raw_body
            if read_pruning_enabled() and goal_hint:
                pruned = prune_read_result(raw_body, goal_hint)
            if read_cache is not None:
                read_cache.put(path, pruned)
            if len(raw_body.encode("utf-8")) >= max_tool_result_bytes():
                truncated = True
            if " [rex: tool output truncated]" in raw_body:
                truncated = True
            if pruned != raw_body:
                result = format_delimited_tool_result_for_prompt(TOOL_READ, pruned)
        return ok, result, truncated

    if tool == TOOL_LIST:
        path = str(args.get("path", "")).strip()
        ok, result = client.list_dir(path, mode)
        return ok, result, False

    if tool == TOOL_WRITE:
        path = str(args.get("path", "")).strip()
        if not path:
            return False, "fs.write requires path", False
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
                return False, patched, False
            ok, msg = client.write_file(path, patched, mode)
            return ok, msg if ok else msg, False
        if content is None:
            return False, "fs.write requires content or diff", False
        content_str = str(content)
        ok_read, existing = client.read_file(path, mode)
        if not ok_read:
            existing = ""
        else:
            existing = strip_tool_result_delimiters(existing)
        reject = reject_whole_file_write(path, content_str, existing)
        if reject:
            return False, reject, False
        ok, msg = client.write_file(path, content_str, mode)
        return ok, msg if ok else msg, False

    if tool == TOOL_EXEC:
        command = str(args.get("command", "")).strip()
        if not command:
            return False, "exec.shell requires command", False
        ok, result = client.exec_shell(command, mode)
        return ok, result, False

    if tool == TOOL_PLAN_SAVE:
        path = normalize_plan_save_path(str(args.get("path", "")))
        content = str(args.get("content", ""))
        if not path:
            return False, "plan.save requires path", False
        if not content.strip():
            return False, "plan.save requires content", False
        ok, msg = client.save_plan(path, content, mode)
        return ok, msg if ok else msg, False

    return False, f"Unknown tool: {tool}", False


def format_delimited_tool_result_for_prompt(tool: str, body: str) -> str:
    """Re-wrap stripped body for LLM scratch when sidecar re-processed content."""
    return f"<<TOOL_RESULT:{tool}>>\n{body}\n<<END>>"


def format_tool_status(call: ToolCall, ok: bool, result: str) -> str:
    return f"\n[tool {call.tool} {'ok' if ok else 'error'}]\n{result}\n"

"""Tool registry, ReAct JSON protocol, and broker execution."""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from typing import Any

from rex_agent.broker import BrokerClient

TOOL_READ = "fs.read"
TOOL_LIST = "fs.list"
TOOL_WRITE = "fs.write"
TOOL_EXEC = "exec.shell"

TOOLS_BY_MODE: dict[str, frozenset[str]] = {
    "ask": frozenset(),
    "plan": frozenset({TOOL_READ, TOOL_LIST}),
    "agent": frozenset({TOOL_READ, TOOL_LIST, TOOL_WRITE, TOOL_EXEC}),
}


@dataclass(frozen=True)
class ToolCall:
    tool: str
    args: dict[str, Any]


@dataclass(frozen=True)
class ParsedModelOutput:
    kind: str  # "final" | "tool" | "error"
    answer: str = ""
    tool_call: ToolCall | None = None
    message: str = ""


def tools_for_mode(mode: str) -> frozenset[str]:
    normalized = (mode or "ask").strip().lower() or "ask"
    return TOOLS_BY_MODE.get(normalized, TOOLS_BY_MODE["ask"])


def system_prompt_for_tools(mode: str) -> str:
    allowed = tools_for_mode(mode)
    if not allowed:
        return (
            "You are a helpful assistant. Respond with your final answer as plain text. "
            "Do not request tools."
        )
    tool_docs = []
    if TOOL_READ in allowed:
        tool_docs.append(
            f'- "{TOOL_READ}": args {{"path": "<relative path>"}}'
        )
    if TOOL_LIST in allowed:
        tool_docs.append(
            f'- "{TOOL_LIST}": args {{"path": "<relative dir or empty for root>"}}'
        )
    if TOOL_WRITE in allowed:
        tool_docs.append(
            f'- "{TOOL_WRITE}": args {{"path": "<relative path>", "content": "<text>"}}'
        )
    if TOOL_EXEC in allowed:
        tool_docs.append(
            f'- "{TOOL_EXEC}": args {{"command": "<shell command>"}}'
        )
    tools_block = "\n".join(tool_docs)
    return (
        "You are a development agent. Use at most one tool per step.\n"
        "When you need a tool, respond with ONLY a JSON object on one line:\n"
        '{"type":"tool","tool":"<name>","args":{...}}\n'
        "When you are done, respond with ONLY:\n"
        '{"type":"final","answer":"<your reply>"}\n'
        "Allowed tools:\n"
        f"{tools_block}\n"
        "If the user message already contains file contents, do not re-read those paths."
    )


def _extract_json_object(text: str) -> str | None:
    stripped = text.strip()
    if stripped.startswith("{") and stripped.endswith("}"):
        return stripped
    match = re.search(r"\{[^{}]*\}", stripped, re.DOTALL)
    if match:
        return match.group(0)
    return None


def parse_model_output(text: str, mode: str) -> ParsedModelOutput:
    allowed = tools_for_mode(mode)
    raw = text.strip()
    if not raw:
        return ParsedModelOutput(kind="error", message="Model returned an empty response.")

    blob = _extract_json_object(raw)
    if blob is None:
        if not allowed:
            return ParsedModelOutput(kind="final", answer=raw)
        return ParsedModelOutput(
            kind="final",
            answer=raw,
        )

    try:
        payload = json.loads(blob)
    except json.JSONDecodeError:
        return ParsedModelOutput(
            kind="error",
            message="Could not parse model output as JSON. Reply with a final answer or valid tool JSON.",
        )

    kind = str(payload.get("type", "")).strip().lower()
    if kind == "final":
        answer = str(payload.get("answer", "")).strip()
        if not answer:
            return ParsedModelOutput(kind="error", message="Final answer JSON must include a non-empty answer.")
        return ParsedModelOutput(kind="final", answer=answer)

    if kind == "tool":
        tool = str(payload.get("tool", "")).strip()
        args = payload.get("args")
        if not isinstance(args, dict):
            return ParsedModelOutput(kind="error", message="Tool call JSON must include an args object.")
        if tool not in allowed:
            return ParsedModelOutput(
                kind="error",
                message=f"Tool {tool!r} is not allowed in {mode} mode.",
            )
        return ParsedModelOutput(kind="tool", tool_call=ToolCall(tool=tool, args=args))

    return ParsedModelOutput(
        kind="error",
        message='Model JSON must use type "final" or "tool".',
    )


def execute_tool(
    client: BrokerClient,
    call: ToolCall,
    mode: str,
) -> tuple[bool, str]:
    tool = call.tool
    args = call.args
    if tool == TOOL_READ:
        path = str(args.get("path", "")).strip()
        if not path:
            return False, "fs.read requires path"
        return client.read_file(path, mode)
    if tool == TOOL_LIST:
        path = str(args.get("path", "")).strip()
        return client.list_dir(path, mode)
    if tool == TOOL_WRITE:
        path = str(args.get("path", "")).strip()
        content = str(args.get("content", ""))
        if not path:
            return False, "fs.write requires path"
        return client.write_file(path, content, mode)
    if tool == TOOL_EXEC:
        command = str(args.get("command", "")).strip()
        if not command:
            return False, "exec.shell requires command"
        return client.exec_shell(command, mode)
    return False, f"Unknown tool: {tool}"


def format_tool_status(call: ToolCall, ok: bool, result: str) -> str:
    status = "ok" if ok else "error"
    return f"\n[tool {call.tool} {status}]\n{result}\n"

"""LangGraph tools that call Rex broker RPCs."""

from __future__ import annotations

from langchain_core.tools import tool

from rex_agent.broker_client import RexBrokerClient
from rex_agent.modes import tool_names_for_mode


def build_tools(broker: RexBrokerClient, mode: str) -> list:
    @tool
    async def read_file(path: str) -> str:
        """Read a workspace file via Rex daemon broker."""
        return await broker.read_file(path)

    @tool
    async def list_dir(path: str = ".") -> str:
        """List entries in a workspace directory via Rex daemon broker."""
        entries = await broker.list_dir(path)
        return "\n".join(entries) if entries else "(empty directory)"

    @tool
    async def write_file(path: str, content: str) -> str:
        """Write content to a workspace file via Rex daemon broker."""
        await broker.write_file(path, content)
        return f"wrote {path}"

    @tool
    async def exec_shell(command: str) -> str:
        """Run an allowlisted shell command in the workspace via Rex daemon broker."""
        return await broker.exec_shell(command)

    normalized = (mode or "ask").strip().lower()
    if normalized == "ask":
        return []
    if normalized == "plan":
        return [read_file, list_dir]
    return [read_file, list_dir, write_file, exec_shell]
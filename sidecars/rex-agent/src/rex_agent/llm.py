"""Prompt assembly for broker-only inference."""

from __future__ import annotations

from dataclasses import dataclass, field

from rex_agent.tools import system_prompt_for_tools


@dataclass
class ChatMessage:
    role: str
    content: str


@dataclass
class Conversation:
    mode: str
    messages: list[ChatMessage] = field(default_factory=list)

    def append(self, role: str, content: str) -> None:
        self.messages.append(ChatMessage(role=role, content=content.strip()))

    def to_prompt(self) -> str:
        parts: list[str] = []
        allowed_tools = system_prompt_for_tools(self.mode)
        parts.append(f"[system]\n{allowed_tools}")
        for msg in self.messages:
            parts.append(f"[{msg.role}]\n{msg.content}")
        return "\n\n".join(parts)


def build_initial_conversation(prompt: str, mode: str) -> Conversation:
    conv = Conversation(mode=mode)
    conv.append("user", prompt)
    return conv

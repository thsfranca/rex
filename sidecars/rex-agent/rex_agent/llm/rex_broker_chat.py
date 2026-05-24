"""LangChain chat model backed by Rex BrokerInference."""

from __future__ import annotations

from typing import Any

from langchain_core.language_models.chat_models import BaseChatModel
from langchain_core.messages import AIMessage, BaseMessage, HumanMessage, SystemMessage, ToolMessage
from langchain_core.outputs import ChatGeneration, ChatResult
from pydantic import Field

from rex_agent.broker_client import RexBrokerClient


def _serialize_messages(messages: list[BaseMessage]) -> str:
    lines: list[str] = []
    for message in messages:
        role = message.type
        if isinstance(message, HumanMessage):
            role = "user"
        elif isinstance(message, AIMessage):
            role = "assistant"
        elif isinstance(message, SystemMessage):
            role = "system"
        elif isinstance(message, ToolMessage):
            role = f"tool:{message.name or 'unknown'}"
        lines.append(f"[{role}]\n{message.content}")
    return "\n\n".join(lines)


class RexBrokerChatModel(BaseChatModel):
    broker: RexBrokerClient = Field(exclude=True)
    mode: str = "agent"
    model: str = ""

    @property
    def _llm_type(self) -> str:
        return "rex-broker"

    def _generate(
        self,
        messages: list[BaseMessage],
        stop: list[str] | None = None,
        run_manager: Any = None,
        **kwargs: Any,
    ) -> ChatResult:
        raise NotImplementedError("use async _agenerate for Rex broker model")

    async def _agenerate(
        self,
        messages: list[BaseMessage],
        stop: list[str] | None = None,
        run_manager: Any = None,
        **kwargs: Any,
    ) -> ChatResult:
        prompt = _serialize_messages(messages)
        text = await self.broker.inference(prompt, self.mode, self.model)
        generation = ChatGeneration(message=AIMessage(content=text))
        return ChatResult(generations=[generation])

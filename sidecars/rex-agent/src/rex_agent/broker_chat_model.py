"""RexBrokerChatModel: LangChain BaseChatModel over broker inference."""

from __future__ import annotations

from collections.abc import AsyncIterator, Iterator
from typing import Any, Optional

from langchain_core.language_models.chat_models import BaseChatModel
from langchain_core.messages import AIMessage, BaseMessage, HumanMessage, SystemMessage
from langchain_core.outputs import ChatGeneration, ChatGenerationChunk, ChatResult

from rex_agent.broker import (
    InferenceResult,
    is_interim_fallback,
    legacy_inference_result,
)
from rex_agent.tools import (
    ParsedModelOutput,
    parse_model_output,
    system_prompt_for_tools,
)

MAX_PARSE_RETRIES = 3
_TOOL_JSON_PREFIX = '{"type":"tool"'


def messages_to_prompt(
    messages: list[BaseMessage],
    mode: str,
    daemon_context: str,
    *,
    subagent: str = "orchestrator",
    viewer_summary: str = "",
) -> str:
    """Static prefix first (system + daemon context), volatile suffix last."""
    tool_mode = mode
    if subagent == "viewer":
        tool_mode = "plan"
    elif subagent == "editor":
        tool_mode = "agent"

    parts: list[str] = [
        f"[system]\n{system_prompt_for_tools(tool_mode, subagent=subagent)}"
    ]
    if viewer_summary and subagent == "editor":
        parts.append(f"[system]\nExploration summary:\n{viewer_summary}")

    if daemon_context.strip():
        parts.append(f"[user]\n{daemon_context.strip()}")

    for msg in messages:
        role = _message_role(msg)
        content = _message_content(msg)
        if content:
            parts.append(f"[{role}]\n{content}")

    return "\n\n".join(parts)


def _message_role(msg: BaseMessage) -> str:
    if isinstance(msg, SystemMessage):
        return "system"
    if isinstance(msg, AIMessage):
        return "assistant"
    if isinstance(msg, HumanMessage):
        return "user"
    return getattr(msg, "type", "user")


def _message_content(msg: BaseMessage) -> str:
    content = msg.content
    if isinstance(content, str):
        return content.strip()
    if isinstance(content, list):
        return " ".join(str(part) for part in content).strip()
    return str(content).strip()


def messages_to_chat_messages(
    messages: list[BaseMessage],
    mode: str,
    daemon_context: str,
    *,
    subagent: str = "orchestrator",
    viewer_summary: str = "",
) -> list[Any]:
    """Structured chat history for native BrokerInference (R038)."""
    try:
        from rex.v1 import rex_pb2
    except ImportError as exc:  # pragma: no cover
        raise ImportError(
            "rex.v1 protobuf stubs not found. Run `rex proto install`."
        ) from exc

    tool_mode = mode
    if subagent == "viewer":
        tool_mode = "plan"
    elif subagent == "editor":
        tool_mode = "agent"

    chat: list[Any] = [
        rex_pb2.ChatMessage(
            role="system",
            content=system_prompt_for_tools(tool_mode, subagent=subagent),
        )
    ]
    if viewer_summary and subagent == "editor":
        chat.append(
            rex_pb2.ChatMessage(
                role="system",
                content=f"Exploration summary:\n{viewer_summary}",
            )
        )
    if daemon_context.strip():
        chat.append(
            rex_pb2.ChatMessage(role="user", content=daemon_context.strip())
        )
    for msg in messages:
        content = _message_content(msg)
        if content:
            chat.append(
                rex_pb2.ChatMessage(role=_message_role(msg), content=content)
            )
    return chat


def parse_to_ai_message(text: str, mode: str) -> tuple[AIMessage, ParsedModelOutput]:
    parsed = parse_model_output(text, mode)
    if parsed.kind == "tool" and parsed.tool_call is not None:
        call = parsed.tool_call
        tool_calls = [{"name": call.tool, "args": call.args, "id": f"call_{call.tool}"}]
        return AIMessage(content=text, tool_calls=tool_calls), parsed
    if parsed.kind == "final":
        return AIMessage(content=parsed.answer), parsed
    return AIMessage(content=text), parsed


def route_inference_result(
    result: InferenceResult, mode: str
) -> tuple[AIMessage, ParsedModelOutput | None]:
    """Apply NATIVE_TOOL_CALLING response routing table."""
    if result.tool_calls:
        lc_tool_calls = [
            {
                "name": call.tool,
                "args": call.args,
                "id": f"call_{call.tool}",
            }
            for call in result.tool_calls
        ]
        return AIMessage(content="", tool_calls=lc_tool_calls), None

    ai, parsed = parse_to_ai_message(result.effective_text(), mode)
    return ai, parsed


def _is_tool_json(text: str) -> bool:
    stripped = text.strip()
    return stripped.startswith(_TOOL_JSON_PREFIX) or (
        stripped.startswith("{") and '"type":"tool"' in stripped
    )


def stream_visible_text(raw: str) -> Iterator[str]:
    """Yield user-visible segments, buffering JSON tool prefix."""
    if _is_tool_json(raw):
        return
    buffer = ""
    for char in raw:
        buffer += char
        if _is_tool_json(buffer):
            buffer = ""
            continue
        yield char
        buffer = ""
    if buffer and not _is_tool_json(buffer):
        yield buffer


class RexBrokerChatModel(BaseChatModel):
    """Broker-only chat model with static-prefix prompt ordering."""

    mode: str = "ask"
    model_name: str = ""
    subagent: str = "orchestrator"
    daemon_context: str = ""
    viewer_summary: str = ""
    inference_fn: Any = None

    @property
    def _llm_type(self) -> str:
        return "rex-broker"

    def _call_inference(
        self,
        prompt: str,
        *,
        messages: list[Any] | None = None,
        tools: list[Any] | None = None,
    ) -> InferenceResult:
        if self.inference_fn is not None:
            try:
                ret = self.inference_fn(
                    prompt,
                    self.mode,
                    self.model_name,
                    messages=messages,
                    tools=tools,
                )
            except TypeError:
                ret = self.inference_fn(prompt, self.mode, self.model_name)
            if isinstance(ret, InferenceResult):
                return ret
            ok, text = ret
            return legacy_inference_result(ok, text)
        raise RuntimeError("inference_fn not configured on RexBrokerChatModel")

    def _generate(
        self,
        messages: list[BaseMessage],
        stop: Optional[list[str]] = None,
        run_manager: Any = None,
        **kwargs: Any,
    ) -> ChatResult:
        prompt = messages_to_prompt(
            messages,
            self.mode,
            self.daemon_context,
            subagent=self.subagent,
            viewer_summary=self.viewer_summary,
        )
        chat_messages = messages_to_chat_messages(
            messages,
            self.mode,
            self.daemon_context,
            subagent=self.subagent,
            viewer_summary=self.viewer_summary,
        )
        tool_specs: list[Any] = []
        try:
            from rex_agent.tools import tool_specs_for_subagent

            tool_specs = tool_specs_for_subagent(self.subagent, self.mode)
        except ImportError:
            pass
        result = self._call_inference(
            prompt,
            messages=chat_messages,
            tools=tool_specs or None,
        )
        if tool_specs and is_interim_fallback(result):
            result = self._call_inference(
                prompt,
                messages=chat_messages,
                tools=[],
            )
        if not result.ok:
            ai = AIMessage(content=result.error or "Inference failed.")
            return ChatResult(generations=[ChatGeneration(message=ai)])
        ai, _ = route_inference_result(result, self.mode)
        return ChatResult(generations=[ChatGeneration(message=ai)])

    def _stream(
        self,
        messages: list[BaseMessage],
        stop: Optional[list[str]] = None,
        run_manager: Any = None,
        **kwargs: Any,
    ) -> Iterator[ChatGenerationChunk]:
        prompt = messages_to_prompt(
            messages,
            self.mode,
            self.daemon_context,
            subagent=self.subagent,
            viewer_summary=self.viewer_summary,
        )
        chat_messages = messages_to_chat_messages(
            messages,
            self.mode,
            self.daemon_context,
            subagent=self.subagent,
            viewer_summary=self.viewer_summary,
        )
        tool_specs: list[Any] = []
        try:
            from rex_agent.tools import tool_specs_for_subagent

            tool_specs = tool_specs_for_subagent(self.subagent, self.mode)
        except ImportError:
            pass
        result = self._call_inference(
            prompt,
            messages=chat_messages,
            tools=tool_specs or None,
        )
        if tool_specs and is_interim_fallback(result):
            result = self._call_inference(
                prompt,
                messages=chat_messages,
                tools=[],
            )
        if not result.ok:
            yield ChatGenerationChunk(
                message=AIMessage(content=result.error or "Inference failed.")
            )
            return
        _, parsed = route_inference_result(result, self.mode)
        visible = parsed.answer if parsed and parsed.kind == "final" else ""
        for segment in stream_visible_text(visible):
            yield ChatGenerationChunk(message=AIMessage(content=segment))

    async def _astream(
        self,
        messages: list[BaseMessage],
        stop: Optional[list[str]] = None,
        run_manager: Any = None,
        **kwargs: Any,
    ) -> AsyncIterator[ChatGenerationChunk]:
        for chunk in self._stream(
            messages, stop=stop, run_manager=run_manager, **kwargs
        ):
            yield chunk

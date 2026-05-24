"""Async streaming chunks from LangGraph execution."""

from __future__ import annotations

from collections.abc import AsyncIterator

from langchain_core.messages import HumanMessage
from langgraph.graph import END, MessagesState, StateGraph
from langgraph.prebuilt import ToolNode, tools_condition

from rex_agent.broker_client import RexBrokerClient
from rex_agent.config import max_tool_steps
from rex_agent.llm.rex_broker_chat import RexBrokerChatModel
from rex_agent.tools.broker_tools import build_tools


def build_graph(broker: RexBrokerClient, mode: str, model: str):
    normalized = (mode or "ask").strip().lower() or "ask"
    llm = RexBrokerChatModel(broker=broker, mode=normalized, model=model)
    tools = build_tools(broker, normalized)

    async def call_model(state: MessagesState):
        response = await llm.ainvoke(state["messages"])
        return {"messages": [response]}

    graph = StateGraph(MessagesState)
    graph.add_node("agent", call_model)

    if not tools:
        graph.set_entry_point("agent")
        graph.add_edge("agent", END)
        return graph.compile()

    tool_node = ToolNode(tools)
    graph.add_node("tools", tool_node)
    graph.set_entry_point("agent")
    graph.add_conditional_edges("agent", tools_condition)
    graph.add_edge("tools", "agent")
    return graph.compile()


async def run_turn_stream(
    broker: RexBrokerClient,
    prompt: str,
    mode: str,
    model: str,
) -> AsyncIterator[str]:
    graph = build_graph(broker, mode, model)
    config = {"recursion_limit": max_tool_steps() + 2}
    yielded = False
    async for update in graph.astream(
        {"messages": [HumanMessage(content=prompt)]},
        config=config,
        stream_mode="updates",
    ):
        for node_output in update.values():
            messages = node_output.get("messages", [])
            for message in messages:
                content = getattr(message, "content", "")
                if isinstance(content, str) and content.strip():
                    yielded = True
                    yield content
                elif content:
                    yielded = True
                    yield str(content)
    if not yielded:
        yield "(no assistant output)"

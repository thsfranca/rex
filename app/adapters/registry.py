from __future__ import annotations

from app.adapters.base import ClientAdapter
from app.adapters.default import DefaultAdapter


class AdapterRegistry:
    def __init__(self) -> None:
        self._adapters: dict[str, ClientAdapter] = {}
        self._default = DefaultAdapter()

    def register(self, user_agent_prefix: str, adapter: ClientAdapter) -> None:
        self._adapters[user_agent_prefix.lower()] = adapter

    def get_adapter(self, user_agent: str | None) -> ClientAdapter:
        if user_agent:
            ua_lower = user_agent.lower()
            for prefix, adapter in self._adapters.items():
                if prefix in ua_lower:
                    return adapter
        return self._default

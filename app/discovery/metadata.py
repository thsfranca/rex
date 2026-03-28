from __future__ import annotations

import logging

import litellm

from app.config import Model
from app.discovery.providers import DetectedProvider

logger = logging.getLogger(__name__)


def enrich_model(model_name: str, provider: DetectedProvider) -> Model:
    cost = 0.0
    is_local = provider.is_local
    max_context_window = None
    supports_function_calling = False
    supports_reasoning = False
    supports_vision = False

    try:
        info = litellm.get_model_info(model_name)
        input_cost = info.get("input_cost_per_token", 0.0) or 0.0
        cost = input_cost * 1000
        max_context_window = info.get("max_input_tokens") or info.get("max_tokens")
        supports_function_calling = bool(info.get("supports_function_calling"))
        supports_reasoning = bool(info.get("supports_reasoning"))
        supports_vision = bool(info.get("supports_vision"))
    except Exception:
        logger.debug("No LiteLLM metadata for %s, using defaults", model_name)

    return Model(
        name=model_name,
        api_key=provider.api_key,
        api_base=provider.api_base,
        cost_per_1k_input=cost,
        is_local=is_local,
        max_context_window=max_context_window,
        supports_function_calling=supports_function_calling,
        supports_reasoning=supports_reasoning,
        supports_vision=supports_vision,
    )

from __future__ import annotations

import logging

import litellm

from app.config import Model, ModelConfig
from app.discovery.providers import DetectedProvider

logger = logging.getLogger(__name__)


def _lookup_capabilities(model_name: str) -> dict:
    try:
        info = litellm.get_model_info(model_name)
        return {
            "cost_per_1k_input": (info.get("input_cost_per_token", 0.0) or 0.0) * 1000,
            "max_context_window": info.get("max_input_tokens") or info.get("max_tokens"),
            "supports_function_calling": bool(info.get("supports_function_calling")),
            "supports_reasoning": bool(info.get("supports_reasoning")),
            "supports_vision": bool(info.get("supports_vision")),
        }
    except Exception:
        logger.debug("No LiteLLM metadata for %s, using defaults", model_name)
        return {}


def enrich_config_model(config_model: ModelConfig) -> Model:
    caps = _lookup_capabilities(config_model.name)
    base = config_model.model_dump()
    if base.get("max_context_window") is None and "max_context_window" in caps:
        base["max_context_window"] = caps["max_context_window"]
    if base.get("cost_per_1k_input", 0.0) == 0.0 and caps.get("cost_per_1k_input", 0.0) > 0.0:
        base["cost_per_1k_input"] = caps["cost_per_1k_input"]
    base["supports_function_calling"] = caps.get("supports_function_calling", False)
    base["supports_reasoning"] = caps.get("supports_reasoning", False)
    base["supports_vision"] = caps.get("supports_vision", False)
    return Model(**base)


def enrich_model(model_name: str, provider: DetectedProvider) -> Model:
    caps = _lookup_capabilities(model_name)
    return Model(
        name=model_name,
        api_key=provider.api_key,
        api_base=provider.api_base,
        cost_per_1k_input=caps.get("cost_per_1k_input", 0.0),
        is_local=provider.is_local,
        max_context_window=caps.get("max_context_window"),
        supports_function_calling=caps.get("supports_function_calling", False),
        supports_reasoning=caps.get("supports_reasoning", False),
        supports_vision=caps.get("supports_vision", False),
    )

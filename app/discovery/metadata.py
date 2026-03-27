from __future__ import annotations

import logging

import litellm

from app.config import ModelConfig
from app.discovery.providers import DetectedProvider

logger = logging.getLogger(__name__)


def enrich_model(model_name: str, provider: DetectedProvider) -> ModelConfig:
    cost = 0.0
    is_local = provider.is_local

    try:
        info = litellm.get_model_info(model_name)
        input_cost = info.get("input_cost_per_token", 0.0) or 0.0
        cost = input_cost * 1000
    except Exception:
        logger.debug("No LiteLLM metadata for %s, using defaults", model_name)

    return ModelConfig(
        name=model_name,
        api_key=provider.api_key,
        api_base=provider.api_base,
        cost_per_1k_input=cost,
        is_local=is_local,
    )

from __future__ import annotations

import logging
import sys

from app.config import ModelConfig, Settings
from app.discovery.metadata import enrich_model
from app.discovery.models import list_models_for_provider
from app.discovery.providers import KNOWN_PROVIDERS, detect_providers
from app.router.registry import ModelRegistry

logger = logging.getLogger(__name__)


async def build_registry(config: Settings | None) -> tuple[ModelRegistry, Settings]:
    settings = config if config is not None else Settings()

    providers = await detect_providers()

    discovered: dict[str, ModelConfig] = {}
    for provider in providers:
        model_names = await list_models_for_provider(provider)
        for name in model_names:
            model = enrich_model(name, provider)
            discovered[name] = model
        logger.info("Discovered %d models from %s", len(model_names), provider.prefix)

    for manual_model in settings.models:
        discovered[manual_model.name] = manual_model

    if not discovered:
        env_vars = ", ".join(sorted(KNOWN_PROVIDERS.keys()))
        logger.error(
            "No models found. Set one of these environment variables: %s "
            "or provide models in config.yaml",
            env_vars,
        )
        sys.exit(1)

    all_models = list(discovered.values())
    logger.info("Total models in registry: %d", len(all_models))

    return ModelRegistry(all_models), settings

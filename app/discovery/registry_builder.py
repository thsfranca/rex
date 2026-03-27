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

    registry: dict[str, ModelConfig] = {}

    config_names: set[str] = set()
    for manual_model in settings.models:
        registry[manual_model.name] = manual_model
        config_names.add(manual_model.name)

    if config_names:
        logger.info("Config defines %d model(s): %s", len(config_names), ", ".join(config_names))

    providers = await detect_providers()

    supplemented = 0
    for provider in providers:
        model_names = await list_models_for_provider(provider)
        for name in model_names:
            if name not in registry:
                registry[name] = enrich_model(name, provider)
                supplemented += 1
        logger.info("Discovered %d models from %s", len(model_names), provider.prefix)

    if config_names and supplemented > 0:
        logger.info("Auto-discovery supplemented %d additional model(s)", supplemented)

    if not registry:
        env_vars = ", ".join(sorted(KNOWN_PROVIDERS.keys()))
        logger.error(
            "No models found. Set one of these environment variables: %s "
            "or provide models in config.yaml",
            env_vars,
        )
        sys.exit(1)

    all_models = list(registry.values())
    logger.info("Total models in registry: %d", len(all_models))

    return ModelRegistry(all_models), settings

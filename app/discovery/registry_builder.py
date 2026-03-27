from __future__ import annotations

import logging
import os
import sys

from app.config import ModelConfig, ProviderConfig, Settings
from app.discovery.metadata import enrich_model
from app.discovery.models import list_models_for_provider
from app.discovery.providers import KNOWN_PROVIDERS, DetectedProvider, detect_providers
from app.router.registry import ModelRegistry

logger = logging.getLogger(__name__)


def _resolve_config_providers(
    config_providers: list[ProviderConfig],
) -> list[DetectedProvider]:
    resolved = []
    for p in config_providers:
        api_base = p.api_base
        if api_base is None and p.api_base_env:
            api_base = os.environ.get(p.api_base_env)
            if api_base is None:
                logger.warning(
                    "Provider %s: env var %s not set, skipping",
                    p.prefix,
                    p.api_base_env,
                )
                continue
        if api_base is None:
            logger.warning(
                "Provider %s: no api_base or api_base_env set, skipping",
                p.prefix,
            )
            continue

        api_key = p.api_key
        if api_key is None and p.api_key_env:
            api_key = os.environ.get(p.api_key_env)
            if api_key is None:
                logger.warning(
                    "Provider %s: env var %s not set, skipping",
                    p.prefix,
                    p.api_key_env,
                )
                continue
        resolved.append(DetectedProvider(prefix=p.prefix, api_key=api_key, api_base=api_base))
    return resolved


async def build_registry(config: Settings | None) -> tuple[ModelRegistry, Settings]:
    settings = config if config is not None else Settings()

    registry: dict[str, ModelConfig] = {}

    config_names: set[str] = set()
    for manual_model in settings.models:
        registry[manual_model.name] = manual_model
        config_names.add(manual_model.name)

    if config_names:
        logger.info("Config defines %d model(s): %s", len(config_names), ", ".join(config_names))

    config_provider_list = _resolve_config_providers(settings.providers)
    config_prefixes = {p.prefix for p in config_provider_list}

    if config_provider_list:
        logger.info(
            "Config defines %d provider(s): %s",
            len(config_provider_list),
            ", ".join(config_prefixes),
        )

    auto_providers = await detect_providers()
    auto_providers = [p for p in auto_providers if p.prefix not in config_prefixes]

    all_providers = config_provider_list + auto_providers

    supplemented = 0
    for provider in all_providers:
        model_names = await list_models_for_provider(provider)
        for name in model_names:
            if name not in registry:
                registry[name] = enrich_model(name, provider)
                supplemented += 1
        logger.info("Discovered %d models from %s", len(model_names), provider.prefix)

    if (config_names or config_provider_list) and supplemented > 0:
        logger.info("Discovery found %d model(s)", supplemented)

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

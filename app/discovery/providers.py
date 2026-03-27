from __future__ import annotations

import logging
import os
from dataclasses import dataclass

import httpx

logger = logging.getLogger(__name__)

KNOWN_PROVIDERS: dict[str, str] = {
    "OPENAI_API_KEY": "openai",
    "ANTHROPIC_API_KEY": "anthropic",
    "GROQ_API_KEY": "groq",
    "GEMINI_API_KEY": "gemini",
    "XAI_API_KEY": "xai",
    "TOGETHERAI_API_KEY": "togetherai",
    "MISTRAL_API_KEY": "mistral",
    "COHERE_API_KEY": "cohere",
}

OLLAMA_DEFAULT_BASE = "http://localhost:11434"


@dataclass
class DetectedProvider:
    prefix: str
    api_key: str | None = None
    api_base: str | None = None
    is_local: bool = False


def detect_cloud_providers() -> list[DetectedProvider]:
    providers = []
    for env_var, prefix in KNOWN_PROVIDERS.items():
        api_key = os.environ.get(env_var)
        if api_key:
            providers.append(DetectedProvider(prefix=prefix, api_key=api_key))
            logger.info("Detected provider: %s (via %s)", prefix, env_var)
    return providers


async def probe_ollama(base_url: str = OLLAMA_DEFAULT_BASE) -> DetectedProvider | None:
    try:
        async with httpx.AsyncClient(timeout=3.0) as client:
            response = await client.get(f"{base_url}/api/tags")
            if response.status_code == 200:
                logger.info("Detected local Ollama at %s", base_url)
                return DetectedProvider(
                    prefix="ollama",
                    api_base=base_url,
                    is_local=True,
                )
    except httpx.ConnectError:
        logger.debug("Ollama not found at %s", base_url)
    except Exception as e:
        logger.debug("Ollama probe failed at %s: %s", base_url, e)
    return None


async def detect_providers() -> list[DetectedProvider]:
    providers = detect_cloud_providers()
    ollama = await probe_ollama()
    if ollama:
        providers.append(ollama)
    return providers

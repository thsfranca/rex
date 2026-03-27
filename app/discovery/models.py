from __future__ import annotations

import logging

import httpx

from app.discovery.providers import DetectedProvider

logger = logging.getLogger(__name__)

CLOUD_MODELS_ENDPOINTS: dict[str, str] = {
    "openai": "https://api.openai.com/v1/models",
    "anthropic": "https://api.anthropic.com/v1/models",
    "groq": "https://api.groq.com/openai/v1/models",
    "mistral": "https://api.mistral.ai/v1/models",
    "togetherai": "https://api.together.xyz/v1/models",
    "cohere": "https://api.cohere.com/v1/models",
    "xai": "https://api.x.ai/v1/models",
}


async def _list_openai_compatible(endpoint: str, api_key: str, prefix: str) -> list[str]:
    headers = {"Authorization": f"Bearer {api_key}"}
    try:
        async with httpx.AsyncClient(timeout=10.0) as client:
            response = await client.get(endpoint, headers=headers)
            if response.status_code != 200:
                logger.warning(
                    "Failed to list models for %s: HTTP %d",
                    prefix,
                    response.status_code,
                )
                return []
            data = response.json()
            models = data.get("data", [])
            return [f"{prefix}/{m['id']}" for m in models if "id" in m]
    except Exception as e:
        logger.warning("Failed to list models for %s: %s", prefix, e)
        return []


async def _list_anthropic_models(api_key: str) -> list[str]:
    headers = {
        "x-api-key": api_key,
        "anthropic-version": "2023-06-01",
    }
    try:
        async with httpx.AsyncClient(timeout=10.0) as client:
            response = await client.get("https://api.anthropic.com/v1/models", headers=headers)
            if response.status_code != 200:
                logger.warning(
                    "Failed to list Anthropic models: HTTP %d",
                    response.status_code,
                )
                return []
            data = response.json()
            models = data.get("data", [])
            return [f"anthropic/{m['id']}" for m in models if "id" in m]
    except Exception as e:
        logger.warning("Failed to list Anthropic models: %s", e)
        return []


async def _list_ollama_models(api_base: str) -> list[str]:
    try:
        async with httpx.AsyncClient(timeout=10.0) as client:
            response = await client.get(f"{api_base}/api/tags")
            if response.status_code != 200:
                logger.warning("Failed to list Ollama models: HTTP %d", response.status_code)
                return []
            data = response.json()
            models = data.get("models", [])
            return [f"ollama/{m['name']}" for m in models if "name" in m]
    except Exception as e:
        logger.warning("Failed to list Ollama models: %s", e)
        return []


async def _list_gemini_models(api_key: str) -> list[str]:
    try:
        async with httpx.AsyncClient(timeout=10.0) as client:
            response = await client.get(
                "https://generativelanguage.googleapis.com/v1beta/models",
                params={"key": api_key},
            )
            if response.status_code != 200:
                logger.warning("Failed to list Gemini models: HTTP %d", response.status_code)
                return []
            data = response.json()
            models = data.get("models", [])
            return [f"gemini/{m['name'].removeprefix('models/')}" for m in models if "name" in m]
    except Exception as e:
        logger.warning("Failed to list Gemini models: %s", e)
        return []


async def list_models_for_provider(provider: DetectedProvider) -> list[str]:
    if provider.prefix == "ollama":
        return await _list_ollama_models(provider.api_base or "http://localhost:11434")

    if provider.prefix == "anthropic":
        return await _list_anthropic_models(provider.api_key)

    if provider.prefix == "gemini":
        return await _list_gemini_models(provider.api_key)

    endpoint = CLOUD_MODELS_ENDPOINTS.get(provider.prefix)
    if endpoint and provider.api_key:
        return await _list_openai_compatible(endpoint, provider.api_key, provider.prefix)

    logger.warning("No model listing strategy for provider: %s", provider.prefix)
    return []

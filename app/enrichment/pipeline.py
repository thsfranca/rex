from __future__ import annotations

import logging
from typing import Protocol

from app.enrichment.context import EnrichmentContext

logger = logging.getLogger(__name__)


class Enricher(Protocol):
    def enrich(self, context: EnrichmentContext) -> EnrichmentContext: ...


class EnrichmentPipeline:
    def __init__(self, enrichers: list[Enricher]) -> None:
        self._enrichers = enrichers

    def run(self, context: EnrichmentContext) -> EnrichmentContext:
        for enricher in self._enrichers:
            try:
                context = enricher.enrich(context)
            except Exception:
                logger.exception(
                    "Enricher %s failed, passing context through",
                    type(enricher).__name__,
                )
        return context

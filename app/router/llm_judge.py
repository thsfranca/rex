from __future__ import annotations

import json
import logging
from dataclasses import dataclass

import litellm

from app.router.categories import TaskCategory
from app.utils import extract_last_user_text

logger = logging.getLogger(__name__)

_VALID_CATEGORIES = {c.value for c in TaskCategory}

JUDGE_SYSTEM_PROMPT = (
    "You are a coding task classifier. "
    "Analyze the user's message and classify it into exactly one category.\n\n"
    "Valid categories: " + ", ".join(sorted(_VALID_CATEGORIES)) + "\n\n"
    "Respond with a JSON object containing:\n"
    '- "category": one of the valid categories listed above\n\n'
    "Respond ONLY with the JSON object, no other text."
)


@dataclass(frozen=True)
class JudgeResult:
    category: TaskCategory


def _parse_judge_response(content: str) -> JudgeResult | None:
    try:
        data = json.loads(content)
    except (json.JSONDecodeError, TypeError):
        logger.warning("LLM judge returned invalid JSON: %s", str(content)[:200])
        return None

    category_str = data.get("category")
    if category_str not in _VALID_CATEGORIES:
        logger.warning("LLM judge returned invalid category: %s", category_str)
        return None

    return JudgeResult(category=TaskCategory(category_str))


class LLMJudge:
    def __init__(self, model: str) -> None:
        self._model = model

    @property
    def model(self) -> str:
        return self._model

    async def classify(self, messages: list[dict]) -> JudgeResult | None:
        user_text = extract_last_user_text(messages)
        if not user_text.strip():
            return None

        try:
            response = await litellm.acompletion(
                model=self._model,
                messages=[
                    {"role": "system", "content": JUDGE_SYSTEM_PROMPT},
                    {"role": "user", "content": user_text},
                ],
                temperature=0.0,
                max_tokens=100,
            )
            content = response.choices[0].message.content
            return _parse_judge_response(content)
        except Exception as e:
            logger.warning("LLM judge call failed: %s", e)
            return None

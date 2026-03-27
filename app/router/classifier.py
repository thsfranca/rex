from __future__ import annotations

import re
from dataclasses import dataclass, field

from app.router.categories import TaskCategory
from app.utils import extract_last_user_text

_KEYWORD_PATTERNS: dict[TaskCategory, list[re.Pattern]] = {
    TaskCategory.DEBUGGING: [
        re.compile(
            r"\b(error|bug|fix|crash|traceback|exception" r"|stack\s*trace|segfault|failing)\b",
            re.I,
        ),
    ],
    TaskCategory.REFACTORING: [
        re.compile(
            r"\b(refactor|clean\s*up|simplify|restructure"
            r"|reorganize|extract\s+(method|function|class))\b",
            re.I,
        ),
    ],
    TaskCategory.OPTIMIZATION: [
        re.compile(
            r"\b(faster|performance|optimize|memory"
            r"|efficient|speed\s*up|slow|bottleneck|profil)\b",
            re.I,
        ),
    ],
    TaskCategory.TEST_GENERATION: [
        re.compile(
            r"\b(write\s+tests?|add\s+tests?|unit\s+tests?"
            r"|test\s+cases?|spec|coverage|pytest|jest)\b",
            re.I,
        ),
    ],
    TaskCategory.EXPLANATION: [
        re.compile(
            r"\b(explain|what\s+does|how\s+does"
            r"|why\s+(does|is|do)|walk\s+me\s+through|what\s+is)\b",
            re.I,
        ),
    ],
    TaskCategory.DOCUMENTATION: [
        re.compile(
            r"\b(document|docstring|readme" r"|api\s+docs?|jsdoc|comment|changelog)\b",
            re.I,
        ),
    ],
    TaskCategory.CODE_REVIEW: [
        re.compile(
            r"\b(review|is\s+this\s+correct|what'?s\s+wrong" r"|security|vulnerability|audit)\b",
            re.I,
        ),
    ],
    TaskCategory.GENERATION: [
        re.compile(
            r"\b(create|implement|build|write|generate" r"|scaffold|boilerplate|add\s+a)\b",
            re.I,
        ),
    ],
    TaskCategory.MIGRATION: [
        re.compile(
            r"\b(upgrade|migrate|convert\s+to" r"|update\s+from|port\s+to|switch\s+from)\b",
            re.I,
        ),
    ],
}

_CATEGORY_PRIORITY = [
    TaskCategory.DEBUGGING,
    TaskCategory.TEST_GENERATION,
    TaskCategory.REFACTORING,
    TaskCategory.CODE_REVIEW,
    TaskCategory.OPTIMIZATION,
    TaskCategory.MIGRATION,
    TaskCategory.EXPLANATION,
    TaskCategory.DOCUMENTATION,
    TaskCategory.GENERATION,
]

_STACK_TRACE_PATTERN = re.compile(
    r"(Traceback \(most recent call last\)"
    r"|at .+\(.+:\d+:\d+\)"
    r"|File \".+\", line \d+"
    r"|^\s+at\s+)",
    re.M,
)

_CODE_BLOCK_PATTERN = re.compile(r"```[\s\S]*?```")

_CODE_HEAVY_CATEGORIES = frozenset(
    {
        TaskCategory.DEBUGGING,
        TaskCategory.REFACTORING,
        TaskCategory.OPTIMIZATION,
        TaskCategory.TEST_GENERATION,
        TaskCategory.CODE_REVIEW,
        TaskCategory.GENERATION,
    }
)

_LONG_CONTEXT_CATEGORIES = frozenset(
    {
        TaskCategory.REFACTORING,
        TaskCategory.CODE_REVIEW,
        TaskCategory.GENERATION,
        TaskCategory.TEST_GENERATION,
    }
)


@dataclass(frozen=True)
class ClassificationResult:
    category: TaskCategory
    confidence: float
    scores: dict[TaskCategory, float] = field(default_factory=dict)


def _has_stack_trace(text: str) -> bool:
    return bool(_STACK_TRACE_PATTERN.search(text))


def _code_block_ratio(text: str) -> float:
    if not text:
        return 0.0
    code_blocks = _CODE_BLOCK_PATTERN.findall(text)
    code_length = sum(len(block) for block in code_blocks)
    return code_length / len(text)


def _count_keyword_matches(text: str, category: TaskCategory) -> int:
    patterns = _KEYWORD_PATTERNS.get(category, [])
    return sum(len(p.findall(text)) for p in patterns)


def classify(messages: list[dict]) -> ClassificationResult:
    text = extract_last_user_text(messages)

    if not text.strip():
        return ClassificationResult(category=TaskCategory.GENERAL, confidence=0.0)

    scores: dict[TaskCategory, float] = {}

    for category in _CATEGORY_PRIORITY:
        match_count = _count_keyword_matches(text, category)
        if match_count > 0:
            scores[category] = min(0.4 + match_count * 0.2, 1.0)

    if _has_stack_trace(text):
        scores[TaskCategory.DEBUGGING] = scores.get(TaskCategory.DEBUGGING, 0.0) + 0.3

    code_ratio = _code_block_ratio(text)
    if code_ratio > 0.3:
        for cat in _CODE_HEAVY_CATEGORIES:
            if cat in scores:
                scores[cat] += 0.1

    prompt_length = len(text)
    if prompt_length > 500:
        for cat in _LONG_CONTEXT_CATEGORIES:
            if cat in scores:
                scores[cat] += 0.1

    if not scores:
        return ClassificationResult(category=TaskCategory.GENERAL, confidence=0.2, scores={})

    best_category = max(
        scores,
        key=lambda c: (scores[c], -_CATEGORY_PRIORITY.index(c)),
    )
    return ClassificationResult(
        category=best_category,
        confidence=min(scores[best_category], 1.0),
        scores=dict(scores),
    )

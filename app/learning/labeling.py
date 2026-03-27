from __future__ import annotations

from dataclasses import dataclass

from app.router.categories import TaskCategory


@dataclass
class RuleStats:
    correct: int = 0
    total: int = 0

    @property
    def accuracy(self) -> float:
        if self.total == 0:
            return 0.5
        return self.correct / self.total

    @property
    def coverage(self) -> float:
        return float(self.total)


class LabelModel:
    def __init__(self) -> None:
        self._rule_stats: dict[str, RuleStats] = {}
        self._previous_accuracies: dict[str, float] = {}
        self._converged = False

    def fit(self, rule_votes_history: list[dict[str, float]]) -> None:
        self._previous_accuracies = {
            rule: stats.accuracy for rule, stats in self._rule_stats.items()
        }
        self._rule_stats = {}

        for votes in rule_votes_history:
            if not votes:
                continue

            majority_category = max(votes, key=votes.get)

            for category, score in votes.items():
                if score <= 0:
                    continue
                if category not in self._rule_stats:
                    self._rule_stats[category] = RuleStats()
                self._rule_stats[category].total += 1
                if category == majority_category:
                    self._rule_stats[category].correct += 1

        max_change = 0.0
        for rule, stats in self._rule_stats.items():
            prev = self._previous_accuracies.get(rule, 0.5)
            max_change = max(max_change, abs(stats.accuracy - prev))

        self._converged = len(self._previous_accuracies) > 0 and max_change < 0.05

    def predict(self, rule_votes: dict[str, float]) -> dict[TaskCategory, float]:
        if not rule_votes:
            return {}

        weighted: dict[TaskCategory, float] = {}
        total_weight = 0.0

        for category_str, score in rule_votes.items():
            if score <= 0:
                continue
            try:
                category = TaskCategory(category_str)
            except ValueError:
                continue

            accuracy = self._rule_stats.get(category_str, RuleStats()).accuracy
            weight = score * accuracy
            weighted[category] = weighted.get(category, 0.0) + weight
            total_weight += weight

        if total_weight == 0:
            return {}

        return {cat: w / total_weight for cat, w in weighted.items()}

    def is_converged(self) -> bool:
        return self._converged

    @property
    def rule_stats(self) -> dict[str, RuleStats]:
        return dict(self._rule_stats)

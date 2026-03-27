from __future__ import annotations

from app.learning.labeling import LabelModel, RuleStats
from app.router.categories import TaskCategory


class TestRuleStats:
    def test_accuracy_with_no_data(self):
        stats = RuleStats()
        assert stats.accuracy == 0.5

    def test_accuracy_all_correct(self):
        stats = RuleStats(correct=10, total=10)
        assert stats.accuracy == 1.0

    def test_accuracy_partial(self):
        stats = RuleStats(correct=7, total=10)
        assert abs(stats.accuracy - 0.7) < 0.01


class TestLabelModel:
    def test_predict_empty_votes_returns_empty(self):
        model = LabelModel()
        assert model.predict({}) == {}

    def test_predict_single_category(self):
        votes_history = [
            {"debugging": 0.8},
            {"debugging": 0.6},
            {"debugging": 0.9},
        ]
        model = LabelModel()
        model.fit(votes_history)

        result = model.predict({"debugging": 0.8})
        assert TaskCategory.DEBUGGING in result
        assert abs(result[TaskCategory.DEBUGGING] - 1.0) < 0.01

    def test_predict_multiple_categories(self):
        votes_history = [
            {"debugging": 0.8, "refactoring": 0.3},
            {"debugging": 0.6, "refactoring": 0.4},
            {"refactoring": 0.9},
        ]
        model = LabelModel()
        model.fit(votes_history)

        result = model.predict({"debugging": 0.5, "refactoring": 0.5})
        assert TaskCategory.DEBUGGING in result
        assert TaskCategory.REFACTORING in result
        total = sum(result.values())
        assert abs(total - 1.0) < 0.01

    def test_predict_ignores_zero_scores(self):
        model = LabelModel()
        model.fit([{"debugging": 0.8}])

        result = model.predict({"debugging": 0.8, "refactoring": 0.0})
        assert TaskCategory.REFACTORING not in result

    def test_predict_ignores_negative_scores(self):
        model = LabelModel()
        model.fit([{"debugging": 0.8}])

        result = model.predict({"debugging": 0.8, "refactoring": -0.1})
        assert TaskCategory.REFACTORING not in result

    def test_predict_ignores_invalid_categories(self):
        model = LabelModel()
        model.fit([{"debugging": 0.8}])

        result = model.predict({"debugging": 0.8, "nonexistent": 0.5})
        assert len(result) == 1

    def test_fit_updates_rule_stats(self):
        model = LabelModel()
        model.fit(
            [
                {"debugging": 0.8, "refactoring": 0.3},
                {"debugging": 0.6},
            ]
        )
        stats = model.rule_stats
        assert "debugging" in stats
        assert stats["debugging"].total >= 2

    def test_not_converged_after_first_fit(self):
        model = LabelModel()
        model.fit([{"debugging": 0.8}])
        assert not model.is_converged()

    def test_converges_with_stable_data(self):
        votes = [{"debugging": 0.8, "refactoring": 0.3}] * 50
        model = LabelModel()
        model.fit(votes)
        model.fit(votes)
        assert model.is_converged()

    def test_not_converged_with_changing_data(self):
        model = LabelModel()
        model.fit([{"debugging": 0.8}] * 50)
        model.fit([{"refactoring": 0.9}] * 50)
        assert not model.is_converged()

    def test_higher_accuracy_rules_get_more_weight(self):
        votes_history = [
            {"debugging": 0.9},
            {"debugging": 0.8},
            {"debugging": 0.7},
            {"debugging": 0.6},
            {"refactoring": 0.5, "debugging": 0.3},
            {"debugging": 0.4, "refactoring": 0.3},
            {"refactoring": 0.5, "debugging": 0.3},
        ]
        model = LabelModel()
        model.fit(votes_history)

        debugging_acc = model.rule_stats["debugging"].accuracy
        refactoring_acc = model.rule_stats["refactoring"].accuracy
        assert debugging_acc > refactoring_acc

        result = model.predict({"debugging": 0.5, "refactoring": 0.5})
        assert result[TaskCategory.DEBUGGING] > result[TaskCategory.REFACTORING]

    def test_predict_before_fit_uses_uniform_accuracy(self):
        model = LabelModel()
        result = model.predict({"debugging": 0.8, "refactoring": 0.6})
        assert TaskCategory.DEBUGGING in result
        assert TaskCategory.REFACTORING in result

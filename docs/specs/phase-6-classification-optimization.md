# Phase 6 — Classification Optimization

> **Goal**: Dramatically reduce classification latency and improve routing accuracy through architectural improvements to the classification chain.

---

## Overview

The classification chain currently runs **sequentially**: heuristics → embedding → centroid → judge. When low-confidence results occur, all downstream classifiers must complete before routing, causing 1000+ ms latency.

Phase 6 introduces **5 top-tier architectural improvements** that reduce latency, improve accuracy, and enable adaptive routing without requiring user feedback.

---

## Improvements

### I. Parallel Classification Chain

**Goal**: Run all classifiers concurrently, return first confident result to avoid expensive judge inference.

**Current**: 1ms (heuristics) + 10ms (embedding) + 1ms (centroid) + 1000ms (judge) = **1012ms sequential**

**After**: max(1ms, 10ms, 1000ms) ≈ **1000ms**, but ~70% of requests get confident heuristics alone → **5-10x faster**

**Details**:
- Fire heuristics, centroid, and judge as parallel tasks
- Return first task that exceeds confidence threshold
- Cancel remaining tasks immediately to save compute
- Fall back to best-confidence result if all complete

**Files Changed**:
- `app/router/engine.py` — `select_model()` method
- `app/router/classifier.py` — Extract heuristics into separate async method

**Tests**:
- Unit: Verify parallel tasks complete independently
- Integration: Verify early-exit skips judge invocation
- Latency: Measure improvement on high/low-confidence requests

**Risk**: Judge task spawned speculatively may consume resources. Mitigate by only spawning if heuristics confidence < 0.5.

---

### II. Structural Token Detection

**Goal**: Detect task structure (stack traces, diffs, error messages) before keywords to boost confidence on common patterns.

**Current**: "what model are you?" → "general" (0.1 confidence) → judge invoked (1000ms)

**After**: Stack traces → "debugging" (0.95 confidence immediately), diffs → "refactoring" (0.95 confidence), no judge needed.

**Details**:
- Add regex patterns for: stack traces, diffs, error messages, code blocks, test assertions
- Pre-score structural signals before running heuristics
- Return early with high confidence if structure is definitive (e.g., "Traceback" → debugging)
- Boost existing classification if structure aligns (e.g., "error" keyword + error density)

**Files Changed**:
- `app/router/classifier.py` — New `extract_structural_features()`, `classify_with_structural_boost()`
- `app/router/categories.py` — (No changes, uses existing categories)

**Tests**:
- Unit: Test each structural pattern detector independently
- Unit: Verify confidence boosting logic
- Integration: Test real stack traces, diffs, error messages

**Risk**: Low. Patterns are precise; false positives are rare. Orthogonal to existing heuristics.

---

### III. Latency-Weighted Model Selection

**Goal**: Track actual response time per model per category, weight model selection by reliability to prefer fast models.

**Current**: Routes only by cost. Haiku always first even if slow on user's hardware.

**After**: If Haiku averages 5s per "general" query but Sonnet averages 1s, Sonnet gets weighted higher. Adaptive routing learns from live usage.

**Details**:
- New `PerformanceTracker` class: records response time per (model, category) pair
- Exponential moving average: 30% new sample, 70% history (avoids oscillation)
- Add `get_latency_score()` method: 100ms → 0.9, 5000ms → 0.1
- In `ModelRegistry.filter_by_requirements()`, combine cost and latency scores
- Weight: 70% cost, 30% latency (tunable)

**Files Changed**:
- `app/learning/performance_tracker.py` — New module
- `app/router/registry.py` — Integrate performance tracking into model filtering
- `app/proxy/handler.py` — Record latency on every decision

**Tests**:
- Unit: Test EMA calculation, latency scoring
- Unit: Test cost/latency weighting formula
- Integration: Simulate 100 requests, verify routing adapts

**Risk**: Exponential weighting may overfit to outliers if not tuned. Counter with clamping (0.1 ≤ score ≤ 1.0).

---

### IV. Confidence-Aware Fallback Chain

**Goal**: Change fallback strategy based on classification confidence. Low confidence → escalate to capable models faster instead of trying cheap models first.

**Current**: Low-confidence "debugging" request tries Haiku, times out, then Sonnet (wasted time).

**After**: Low-confidence request tries Sonnet first. Medium confidence alternates cheap/capable. High confidence tries cheap first.

**Details**:
- Three fallback strategies: "cost" (current), "balanced" (alternate cheap/capable), "capability" (capable first)
- Decision confidence determines which strategy to use:
  - ≥ 0.8: "cost" (high confidence, try cheap)
  - 0.5–0.8: "balanced" (medium confidence, mixed)
  - < 0.5: "capability" (low confidence, prefer capable)
- Implement in `ModelRegistry.fallback_order(model, strategy)`

**Files Changed**:
- `app/router/registry.py` — New `fallback_order()` method with strategy parameter
- `app/proxy/handler.py` — Pass confidence to fallback chain, use strategy-based ordering

**Tests**:
- Unit: Test each fallback strategy ordering
- Integration: Verify low-confidence requests don't timeout on weak models

**Risk**: Low. Fallback chain already exists, just reordering. No breaking changes.

---

### V. Hierarchical Exemplar Clustering

**Goal**: Classify by task **complexity** within category, not just category. A 10-token rename goes to Haiku; a system redesign goes to Sonnet.

**Current**: "refactor" always routes to same model regardless of scope.

**After**: Trivial refactors → Haiku (cheap), complex refactors → Sonnet (capable). 20-30% better cost/quality tradeoff.

**Details**:
- Add `TaskComplexity` enum: TRIVIAL, SIMPLE, MODERATE, COMPLEX
- Build hierarchical exemplar queries per (category, complexity) pair
- Extend `CentroidClassifier` to return (category, complexity) tuple
- Extend `get_requirements()` to return different requirements for TRIVIAL vs COMPLEX
- Router filters models by category + complexity requirements

**Files Changed**:
- `app/router/categories.py` — New `TaskComplexity` enum, hierarchical exemplar queries, updated `get_requirements()`
- `app/learning/centroids.py` — `HierarchicalCentroidClassifier` (or extend existing)
- `app/router/engine.py` — Handle (category, complexity) routing decisions
- `app/router/registry.py` — Filter by category + complexity requirements

**Tests**:
- Unit: Test complexity classification
- Unit: Test requirements logic for each complexity level
- Integration: Test routing of trivial vs complex tasks

**Risk**: Medium. Requires exemplar curation and testing. If complexity classification is wrong, may route trivial tasks to expensive models. Mitigate with conservative thresholds.

---

## Implementation Order (Incremental PRs)

Each PR is a standalone unit that improves a specific concern. PRs build incrementally.

### **PR 1: Structural Token Detection**
- **Scope**: Add structural pattern detection to heuristic classifier
- **Why**: Easiest win (pure regex), no async changes, orthogonal to other improvements
- **Files**: `app/router/classifier.py` (new functions), tests
- **Impact**: 2-3x faster for debugging/diffs
- **Tests**: Unit tests for each pattern, integration tests with real examples

### **PR 2: Parallel Classification Chain**
- **Scope**: Refactor `select_model()` to run classifiers concurrently
- **Why**: Biggest latency improvement; builds on PR 1's structured confidence boosting
- **Files**: `app/router/engine.py`, `app/router/classifier.py` (extract async), tests
- **Impact**: 5-10x faster for high-confidence queries
- **Tests**: Unit tests for concurrent task management, latency benchmarks
- **Dependencies**: PR 1 (optional but recommended for better confidence signals)

### **PR 3: Performance Tracking Foundation**
- **Scope**: Add latency tracking infrastructure (no routing changes yet)
- **Why**: Groundwork for adaptive routing; non-invasive
- **Files**: `app/learning/performance_tracker.py` (new), `app/proxy/handler.py` (record latency), tests
- **Impact**: None yet (data collection only)
- **Tests**: Unit tests for EMA, latency scoring

### **PR 4: Latency-Weighted Model Selection**
- **Scope**: Integrate latency tracking into router
- **Why**: Builds on PR 3; adaptive routing learns from live data
- **Files**: `app/router/registry.py` (integrate performance tracking), tests
- **Impact**: 10-20% latency improvement after warmup (~100 requests)
- **Tests**: Integration tests showing adaptive routing

### **PR 5: Confidence-Aware Fallback**
- **Scope**: Use classification confidence to determine fallback strategy
- **Why**: Prevents wasted attempts on weak models; uses decision confidence from earlier PRs
- **Files**: `app/router/registry.py` (fallback strategies), `app/proxy/handler.py` (use confidence), tests
- **Impact**: Fewer timeouts, faster recovery
- **Tests**: Integration tests with low/medium/high confidence requests

### **PR 6: Hierarchical Exemplars (Phase 6.1)**
- **Scope**: Add complexity dimension to exemplar clustering
- **Why**: Refinement; builds on all prior PRs; requires most tuning
- **Files**: `app/router/categories.py`, `app/learning/centroids.py`, `app/router/engine.py`, tests
- **Impact**: 20-30% cost improvement with better accuracy
- **Tests**: Unit tests for complexity classification, integration tests, cost/quality benchmarks
- **Status**: Optional follow-up; valuable but requires more configuration

---

## Testing Strategy

### Unit Tests
- Each new class/function tested in isolation
- Mock external dependencies (models, embeddings)
- Test edge cases (empty input, timeout, invalid categories)

### Integration Tests
- Full request flow from proxy to router to handler
- Verify classification improvements with real examples
- Latency benchmarks before/after each PR

### Benchmarking
- Track latency per classification path (heuristics, centroid, judge, parallel)
- Track confidence distribution
- Track model selection distribution
- Dashboard: decision logs analyzed with Datasette

---

## Rollout Strategy

### Canary (Phase 6.0–6.1)
- Deploy PR 1–2 to monitor latency improvements
- Monitor for regressions in classification accuracy
- Adjust confidence thresholds if needed

### Full Release (Phase 6.2+)
- Add PR 3–5 for adaptive routing and fallback
- Gather 500+ decisions before tuning hierarchical exemplars
- Deploy PR 6 as optional enhancement

---

## Metrics

- **Classification Latency** (ms): Time from request to routing decision
- **Confidence Distribution**: % of requests at each confidence level
- **Fallback Rate** (%): % of requests that trigger fallback
- **Model Selection**: Distribution of models selected per category
- **Cost**: Total API cost vs baseline
- **Success Rate** (proxy signal): Fallback-triggered → model succeeded

---

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Judge task spawned speculatively consumes resources | Only spawn judge if heuristics < 0.5 |
| Structural patterns have false positives | Use high-precision patterns (e.g., "Traceback" not "error") |
| EMA overfits to outliers | Clamp scores to [0.1, 0.9] |
| Complexity classification is wrong | Conservative thresholds, manual validation |
| Parallel tasks cancel prematurely | Test thoroughly before rollout |

---

## Future Work (Phase 7+)

- **Explicit Feedback**: User ratings to train quality model
- **Bandit Algorithms**: Thompson sampling for exploration
- **Auto-tuning**: Learn model strengths per category beyond cost/latency
- **Conversation Tracking**: Detect follow-ups to measure satisfaction

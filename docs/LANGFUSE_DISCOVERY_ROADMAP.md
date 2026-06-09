# LangFuse discovery roadmap

**Diátaxis role:** explanation — ordered **discovery** tasks before LangFuse integration implementation.

**Status:** **active** — **LF-D01** is the next slice.

**Parent hub:** [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md) · **Main queue:** [ROADMAP.md](ROADMAP.md)

## Purpose

LangFuse is a broad platform. Discovery tasks produce signed-off outputs (mapping tables, spike notes, RC drafts) so implementation PRs (**LF-F01+**, **LF-R01**) do not guess contracts.

## Discovery complete (gate)

**Minimum:** **LF-D01**, **LF-D02**, and **LF-D09** done.

**Parallel after D01:** **LF-D03–LF-D08**, **LF-D10**.

## Discovery tasks

| ID | Theme | Refines | Outputs | Gate to implement |
|----|-------|---------|---------|-------------------|
| **LF-D01** | LangFuse Cloud project + OTLP ingest | LF-F01 | Cloud project; OTEL endpoint URL; key handling doc; confirm Rex `observability.otlp` vs LangFuse OTEL requirements | **LF-F01** may start |
| **LF-D02** | Economics field mapping | LF-F01 | Signed mapping table (see hub); PII rules (metadata-only default) | **LF-F01** attribute contract frozen |
| **LF-D03** | LiteLLM dual export | LF-F02 | Spike config: broker → LiteLLM + `langfuse_otel`; duplicate-token risk note | **LF-F02** |
| **LF-D04** | LangGraph tracing depth | LF-F03 | Spike `CallbackHandler` + `RexBrokerChatModel`; capture gap list | **LF-F03** |
| **LF-D05** | Validation → datasets | LF-F04 | Scenario matrix → LangFuse dataset/run model; retire Rex `runs`/`run_tasks` intent | **LF-F04** |
| **LF-D06** | Prompt CMS boundary | LF-F05 | Layer table: LangFuse vs daemon assembly ([ADR 0012](architecture/decisions/0012-layered-prompt-assemblies.md)) | **LF-F05** |
| **LF-D07** | Quality evaluators | LF-F06 | LLM-as-judge dimensions per validation scenario | **LF-F06** |
| **LF-D08** | Annotation workflow | LF-F07 | Golden-set workflow from agent/extension traces | **LF-F07** |
| **LF-D09** | v1.0 RC replacement | v1.0 tag | **RC-LF1** criteria validated in [V1_0.md](V1_0.md); cancel **RC-S3–S5** | Tag gate honest |
| **LF-D10** | ADR supersession outline | governance | Outline superseding ADR 0021/0025/0026/0027; retain 0010/0020 | Implementation ADR PR |

## Implementation row (post-discovery docs)

| ID | Theme | PR series | Blocked until |
|----|-------|-----------|---------------|
| **LF-R01** | Remove Rex observability dead code (store, read API, Grafana suite) | PR 2 | LangFuse docs pivot merged to `main` |
| **LF-F01** | Daemon OTLP → LangFuse Cloud | PR 3+ | LF-D01, LF-D02; LF-R01 recommended |

## How to run a discovery task

1. Record outputs in the hub or a short note linked from this row (no `docs/research/` tree).
2. Update **Status** in this table when done.
3. Mirror completion in [ROADMAP.md](ROADMAP.md) and [PRIORITIZATION.md](PRIORITIZATION.md).

## Cross-links

- [LANGFUSE_INTEGRATION.md](LANGFUSE_INTEGRATION.md) — features and boundaries
- [ECONOMICS_VALIDATION.md](ECONOMICS_VALIDATION.md) — scenarios for **LF-D05**, **LF-D07**
- [CONFIGURATION.md](CONFIGURATION.md) — `observability` JSON (OTLP keys until **LF-F01**)

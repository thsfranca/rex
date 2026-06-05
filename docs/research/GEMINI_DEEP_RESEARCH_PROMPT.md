# Gemini deep research: REX agent token and format optimization

Use this document in **Gemini** (Deep Research or long-context chat). Attach or paste the companion context file first, then paste the prompt below.

| File | Role |
|------|------|
| [rex-agent-system-context.txt](rex-agent-system-context.txt) | **Attach / paste first** — how REX works today |
| This file | **Copy section "Prompt to paste"** into Gemini |

After Gemini finishes, bring the report back to **Cursor** to turn findings into roadmap items (R027–R033), ADR updates, or implementation slices.

---

## Prompt to paste

```
You are doing deep technical research for the REX open-source project (local dev-assistance stack: Rust daemon, VS Code extension, Python rex-agent sidecar). I attached/pasted REX_AGENT_SYSTEM_CONTEXT — treat it as authoritative for current architecture, constraints, and roadmap IDs.

## Research objective

Produce an implementation-oriented research report to optimize the **software engineering agent** end-to-end: token economics, reliability, and **text/serialization formats** at every boundary (user → daemon → sidecar → LLM → tools → user). The output will be used in Cursor to design features and PR slices — not as marketing copy.

Your prior research may have covered generic formats (TOON, TRON, YAML) without REX-specific boundaries. This pass MUST map every recommendation to REX layers and call out what is irrelevant for REX’s tiny per-step tool payloads vs large per-turn context.

## Mandatory coverage areas

### A. Format and protocol alternatives (be exhaustive)

Compare at minimum:
- Current REX interim protocol: one-line JSON `{"type":"tool"|"final",...}` inside a single HTTP `user` message with embedded `[system]`/`[user]`/`[assistant]` tags
- Provider-native **function calling** / **tools[]** with `strict: true` (OpenAI, Anthropic tool use, Gemini function declarations)
- **Structured outputs** / JSON schema constrained decoding
- Compact serializations: TOON, TRON, “safe YAML”, minimal JSON, CBOR/MessagePack (only if tokenization benefit is evidenced)
- Line-oriented / delimiter protocols (e.g. `TOOL fs.read path=...`, TOML fragments, XML — include why they fail or win for *generation* by LLMs)
- Natural-language tool selection patterns (e.g. YES/NO per tool) and when they beat structured args

For each option provide:
1. **Token profile** — input vs output; static prefix cacheability; multi-turn compounding
2. **Reliability** — parse failure rate, need for retries (REX allows up to 3 parse retries today)
3. **Fit for REX** — daemon single-message HTTP vs future R033 native BrokerInference tools
4. **Migration cost** — proto/daemon/sidecar/extension touch points

Cite primary sources: peer-reviewed or arXiv papers (e.g. “Notation Matters” TOON/TRON agentic benchmarks), official provider docs, and reputable engineering writeups. Mark claims as **evidence-backed** vs **inference**.

### B. REX-specific economics (not generic LLM tips)

Analyze cost drivers in the attached context:
- **Per-turn once**: daemon `effective_prompt` + `[context]` lexical retrieval
- **Per-step repeat**: full `messages_to_prompt()` including system blurb + daemon_context + tool transcripts
- **Tool result bulk**: file reads vs JSON wrapper overhead
- **Subagents**: Viewer vs Editor message filtering, viewer_summary
- **Policies**: agent mode no L1 cache; approvals; max_tool_steps=12

Quantify (ranges OK) which lever typically dominates spend in a 5–12 step agent task on a medium repo. Recommend priority order for REX engineering.

### C. Provider and gateway alignment

REX today: sidecar → BrokerInference → daemon → OpenAI-compatible HTTP (optional LiteLLM gateway per docs/INFERENCE_GATEWAY.md).

Research:
- Best practice for **prompt caching** / prefix stability given REX’s static-first `messages_to_prompt` ordering
- Whether to move tool definitions out of repeated system text into API `tools` (ties to R033)
- Multi-model considerations (local Ollama vs cloud) when format differs

### D. Competitor / industry patterns (brief, factual)

How do these handle tool + context economics (no fanboy tone):
- Cursor agent / rules injection
- GitHub Copilot agent + instructions files
- Claude Code / Anthropic tool format
- Devin-style or SWE-agent patch-only edit patterns

Extract **transferable patterns** that respect REX’s daemon-owned context assembly.

### E. Deliverables for Cursor design (required structure)

End with these sections:

1. **Executive summary** (≤15 bullets)
2. **Recommendation matrix** — table: Option | REX fit (1–5) | Token savings (est.) | Reliability risk | Effort | Milestone (R027–R033 or new ID)
3. **Top 5 changes** — ordered by ROI; each with acceptance criteria measurable in CI or dogfood logs
4. **Format decision** — explicit recommendation: keep JSON-in-text, migrate to native tools, hybrid, or alternative serialization — with decision triggers
5. **Anti-patterns for REX** — what not to do given broker-only sidecar
6. **Benchmark protocol** — how to A/B in REX (metrics: tokens_in/out per successful task, tool_steps, parse_retries, wall time, task success on a fixed golden set)
7. **Open questions** — need Rex maintainer choice
8. **Bibliography** — URLs and paper titles

## Output rules

- Write in English; use markdown headings.
- Be critical: if TOON/TRON saves <5% on REX’s one-line tool calls but risks multi-turn cascades, say so explicitly.
- Do not propose sidecar direct LLM keys or bypassing daemon policy.
- Separate **Phase 1 quick wins** (no proto change) from **Phase 2** (R033 native tools, MCP).
- If evidence conflicts, present both sides and recommend an experiment.

Begin by restating your understanding of REX’s three prompt layers (extension user text, daemon effective_prompt, sidecar messages_to_prompt) in ≤1 paragraph, then proceed with the research.
```

---

## Tips for Gemini

1. Upload **rex-agent-system-context.txt** as a file, or paste it in the first message.
2. Enable **Deep Research** (or equivalent) if available so web and papers are pulled in.
3. Ask Gemini to **search** for: TOON TRON agentic benchmark 2025 2026, OpenAI structured outputs strict function calling tokens, notation matters arxiv agent formats.
4. Save the full report as markdown; in Cursor, reference `docs/AGENT_GRAPH_ARCHITECTURE.md` and `docs/ROADMAP.md` when turning items into slices.

## Findings incorporated (2026-06-04)

Deep-research themes are captured in Rex as:

- [ADR 0023](../architecture/decisions/0023-hybrid-agent-serialization-boundaries.md) — hybrid serialization per boundary; rejected TOON/YAML-gen/CBOR/NLT
- [AGENT_GRAPH_ARCHITECTURE.md](../AGENT_GRAPH_ARCHITECTURE.md) — cost model, token playbook, anti-patterns, microcompaction, **R034** / **R036**
- Roadmap IDs **R034** (raw delimited results, Should), **R036** (TRON schema, Could); program order in [ROADMAP.md](../ROADMAP.md) and [AGENT_DELIVERY_ROADMAP.md](../AGENT_DELIVERY_ROADMAP.md)
- Techythings synthesis: `themes/rex-agent-token-economics/` in the research repository

## After research (Cursor)

- Map recommendations to milestones **R027–R036** before inventing new IDs.
- Update or add an ADR only for accepted cross-cutting decisions (format / broker contract).
- Run economics validation per `docs/ECONOMICS_VALIDATION.md` if proposing token claims.

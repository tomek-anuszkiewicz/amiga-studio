# Autonomous AI Agent Engineering & Pair-Programming Guide

This repository is engineered from the ground up for autonomous AI agent pair-programming with strict architectural guardrails.

> 📖 **Case Study & Engineering Narrative:** For an in-depth retrospective on how this emulator was built with zero hand-written human code through architectural sparring, minimal frame prototyping, and an evolving harness, see [How This Emulator Was Written: Pair-Programming with an AI Agent](how_this_emulator_was_written.md).

---

## 1. Architectural Guardrails & Invariants

All automated modifications and agent sessions must strictly adhere to [`AGENTS.md`](../AGENTS.md) and modularized rules under [`.agents/rules/`](../.agents/rules/):

- **Language Policy (`language-policy.md`):** Strict English for all code, identifiers, comments, documentation, and commits.
- **Audio Voice Transcription (`audio-transcription.md`):** Transcribe spoken user voice recordings at the top of responses before proceeding in English.
- **Strict Path Privacy (`no-external-paths.md`):** Zero external host paths in committed files; use generic placeholders.
- **Zero Host Panics (`performance-and-readability.md`):** No `.unwrap()` or `.expect()` in runtime emulation hot paths.
- **Hardware Efficiency & Readability:** Zero custom macros (`macro_rules!`), zero const-generic instruction handlers, contiguous memory layouts, and zero heap allocations in execution loops.
- **Amiga RAG Knowledge Base (`amiga-rag.md`):** Mandatory pre-task conceptual retrieval and automated reindexing on documentation changes.
- **Graphify AST Knowledge Graph (`graphify.md`):** Consult the code knowledge graph for AST queries and adhere to scoped subtree re-indexing (`crates/` vs `ref_src/`).
- **Information Hierarchy (`information-hierarchy.md`):** Inverted pyramid structure, leading with key architectural conclusions.
- **Practitioner Voice & Tone (`practitioner-voice-and-tone.md`):** Hands-on lead architect persona, in-depth tech blog standard, and zero academic/dissertation jargon.
- **Structural Root-Cause Resolution (`structural-root-cause.md`):** Mandatory structural upstream fixes; strict prohibition of local symptom patches.
- **Strict Scope Discipline (`strict-scope-discipline.md`):** Strict task boundary, minimal necessary diffs, zero unsolicited refactoring, and delivered vs suggested reporting.

---

## 2. Knowledge Retrieval: RAG & Graphify

### A. Domain Hardware Knowledge: Local Vector RAG (`amiga-rag`)
- **Knowledge Base Scope:** Connects to local Qdrant database (`http://localhost:6333`, collection: `amiga`), indexing Commodore Hardware Reference Manuals, M68000 PRMs, technical specs, and design specs under `Obsidian/Amiga/`.
- **Automated Reindexing Trigger ([`amiga-rag.md`](../.agents/rules/amiga-rag.md)):**
  - Reindexing is **fully automated**: whenever hardware documentation, reference guides, or design notes under `Obsidian/Amiga/` are added, modified, or reorganized, incremental reindexing is triggered automatically without requiring manual execution.
  - Powered by a local SHA-256 hash cache (`amiga_rag_cache.json`), re-indexing verifies unchanged files instantly (< 1s) and embeds only modified text.
- **Trusted Data Boundary & Provenance:**
  - The local RAG vector store and Graphify AST graphs operate exclusively on a trusted local boundary.
  - Only authoritative Commodore/Motorola hardware reference manuals and internal design specs should be placed in `Obsidian/Amiga/`.
  - Pinned upstreams in `tools/bootstrap.ps1` and strict architectural isolation between guest 68000 emulation and host LLM prompting prevent indirect prompt injection risks.
- **Query Tools & Manual Override:**
  - Query via MCP tool: `rag_search(query="<topic>", sources=["amiga", "obsidian"])`.
  - Manual / interactive CLI runner: `.\tools\rag\bin\amiga_rag.ps1 "Obsidian/Amiga" --source amiga`.

### B. Code Structure & Relationships: AST Knowledge Graph (`graphify`)
- **What is Indexed:** Graphify parses Abstract Syntax Trees (AST), symbol relationships, and call hierarchies across two distinct codebases:
  - **Active Emulator Crates (`crates/`):** Core Rust workspace (`m68000`, `memory_bus`, `debugger`, `gui`, `config`, `rtc`, `test_runner`).
  - **Reference Emulator Sources (`ref_src/`):** Clean C++ reference implementation (`ref_src/vAmiga`) and external test harnesses.
- **Incremental Knowledge Graph Updates ([`graphify.md`](../.agents/rules/graphify.md)):**
  - Graphify maintains an AST cache that extracts only modified files in 1–2 seconds without LLM calls.
  - After modifications to code in `crates/` or `ref_src/`, run `graphify update .` from the repository root (or via `.\tools\bootstrap.ps1 -Graphify`).
  - This keeps a single unified knowledge graph in `graphify-out/` connecting active emulator crates and reference implementations.
- **Query Tools:**
  - `graphify query "<question>"`: Query symbol dependencies, call hierarchies, and architectural boundaries.
  - `graphify path "<A>" "<B>"`: Trace the shortest dependency or call path between two types or functions.
  - `graphify explain "<concept>"`: Extract a focused subgraph explaining a subsystem or module.

---

## 3. Specialized Agent Skills (`.agents/skills/`)

Operational procedures and recipes are modularized under [`.agents/skills/`](../.agents/skills/) across four primary engineering domains:

### A. CPU & Hardware Emulation
- [`add-m68k-instruction`](../.agents/skills/add-m68k-instruction/SKILL.md): Step-by-step recipe for implementing new M68000 instructions (decoding, CCK micro-steps, inlined CCR flags, and single-step test validation).
- [`m68k-singlestep-test`](../.agents/skills/m68k-singlestep-test/SKILL.md): Diagnostic runbook for running, isolating, and troubleshooting Tom Harte physical silicon single-step hardware test failures.
- [`audit-hardware-quality`](../.agents/skills/audit-hardware-quality/SKILL.md): Comprehensive hardware architectural and silicon fidelity audit covering bus topology, Agnus DMA mastership, passive chip latching, CCK timing, and silicon invariants.

### B. Quality Assurance, Performance & Refactoring
- [`code-review`](../.agents/skills/code-review/SKILL.md): Comprehensive 14-point audit checklist for code quality, file sizes ($\le 800$ lines), inlining, zero runtime panics, and spec compliance.
- [`test-runner`](../.agents/skills/test-runner/SKILL.md): Standardized test execution across all 4 tiers, `.test_results/` snapshot management, and automated differential regression telemetry.
- [`integration-test-sprint`](../.agents/skills/integration-test-sprint/SKILL.md): 4-Iteration Cascading Verification Protocol runbook for multi-chip integration test suites, failure clustering, and 2–3 attempt limits.
- [`synthesize-test-fixes`](../.agents/skills/synthesize-test-fixes/SKILL.md): Post-facto root-cause consolidation using the 3-Column Diagnostic Matrix (`[Symptom] | [Location] | [Mechanism]`) to replace scattered local workarounds with a unified upstream hardware model.
- [`audit-code-quality`](../.agents/skills/audit-code-quality/SKILL.md): Comprehensive on-demand code quality auditor and pruning playbook covering dead code, test-only zombies, minimum visibility leaks, SRP cohesion, inlining, and test parity.
- [`refactor-split-module`](../.agents/skills/refactor-split-module/SKILL.md): Decompose oversized Rust source files (> 800 lines) into cohesive submodules while preserving 3-tier re-exports.
- [`profile-external`](../.agents/skills/profile-external/SKILL.md): Profile Amiga 500 emulator execution hot paths using external sampling profilers (`samply` / Firefox Profiler) and verify throughput against Git-tracked baselines.
- [`egui-vision-debugger`](../.agents/skills/egui-vision-debugger/SKILL.md): Headless visual inspection and autonomous self-healing skill for the `egui` frontend using `gui-inspector` (`egui_kittest` + `wgpu`) and Agent Multimodal Vision to diagnose layout squishing, splitter contention, and focus lifecycles.
- [`capture-gui-screenshot`](../.agents/skills/capture-gui-screenshot/SKILL.md): 1-shot headless screenshot capture and multimodal visual inspection for the Amiga 500 egui Developer Studio.

### C. Architecture, Knowledge & Documentation
- [`obsidian-vault-linking`](../.agents/skills/obsidian-vault-linking/SKILL.md): Enforces Line 1 YAML properties (`tags: [spec, ...]`), inverted pyramid structure, dual-layer linking (inline contextual + bottom structural references), and zero broken links across [Obsidian/Amiga/Design/](../Obsidian/Amiga/Design/).
- [`sync-design-docs`](../.agents/skills/sync-design-docs/SKILL.md): Synchronize Obsidian design specifications with active codebase changes, prune draft code, and update crate graphs.
- [`roadmap-maintenance`](../.agents/skills/roadmap-maintenance/SKILL.md): Systematic maintenance and substrate-first ordering of `ROADMAP.md` with zero retention of completed items.
- [`compact-diary`](../.agents/skills/compact-diary/SKILL.md): Milestone compaction procedure synthesizing older chronological log entries in [DIARY.md](../DIARY.md) into concise architectural digests while preserving key evolutionary rationale and verified results.
- [`describe-diagram-assets`](../.agents/skills/describe-diagram-assets/SKILL.md): Inspect circuit diagrams via multimodal vision, author technical sidecars (`<image>.txt`), and sync RAG cache.
- [`pdf-to-markdown`](../.agents/skills/pdf-to-markdown/SKILL.md): High-fidelity document conversion toolchain (PyMuPDF chapter splitting, figure cropping, SVG vectorization, and table stitching) for technical reference manuals.
- [`index-amiga-rag`](../.agents/skills/index-amiga-rag/SKILL.md): Operational procedure for Qdrant vector reindexing, status verification, and offline visual diagram sidecar maintenance (`<image>.txt`).
- [`author-methodology-doc`](../.agents/skills/author-methodology-doc/SKILL.md): Author, audit, or restructure narrative articles, methodology documents, essays, and retrospective devlogs (e.g. `docs/how_this_emulator_was_written.md`) using the 6-layer Inverted Pyramid hierarchy.
- [`html-to-markdown`](../.agents/skills/html-to-markdown/SKILL.md): Standardized toolchain for converting legacy Word HTML, vintage web documentation, and technical HTML articles into clean, publication-grade Obsidian Markdown with asset extraction, layout unnesting, and anchor link validation.
- [`graphify`](../.agents/skills/graphify/SKILL.md): Persistent code knowledge graph navigation, call hierarchy tracing, and scoped subtree updates (`crates/` vs `ref_src/`).
- [`audit-docs-quality`](../.agents/skills/audit-docs-quality/SKILL.md): Comprehensive documentation, Obsidian vault linking, constitutional size limits, and agent governance quality audit playbook.
- [`audit-semantic-parity`](../.agents/skills/audit-semantic-parity/SKILL.md): Inference-driven bidirectional semantic audit evaluating code-to-docs parity (blind spots, undocumented code) and docs-to-code parity (hallucinations, ghost features, spec drift).

### D. Git & Worktree Orchestration
- [`git-worktree`](../.agents/skills/git-worktree/SKILL.md): Create, synchronize, and tear down isolated Git worktrees with automatic physical copying of ignored test assets and `.env` (Zero NTFS Junctions).
- [`git-resolve-merge`](../.agents/skills/git-resolve-merge/SKILL.md): Resolve 3-way Git merge conflicts holistically in isolated worktrees with mandatory merge commits and regression verification.

---

## 4. Interactive Slash Command Workflows (`.agents/workflows/`)

Developers trigger high-level orchestration directly in the IDE chat UI using slash commands:

| Slash Command | Workflow File | Primary Purpose |
| :--- | :--- | :--- |
| **`/code-review`** | [`code-review.md`](../.agents/workflows/code-review.md) | Comprehensive 14-point pre-commit and milestone architectural compliance audit. |
| **`/audit-code-quality`** | [`audit-code-quality.md`](../.agents/workflows/audit-code-quality.md) | Full-workspace Rust code quality audit (dead code, zombies, visibility, SRP, inlining, test parity). |
| **`/audit-docs-quality`** | [`audit-docs-quality.md`](../.agents/workflows/audit-docs-quality.md) | Full-repository documentation and governance audit (design sync, vault links, size limits, skills catalog). |
| **`/audit-semantic-parity`** | [`audit-semantic-parity.md`](../.agents/workflows/audit-semantic-parity.md) | Inference-driven bidirectional code-to-docs and docs-to-code semantic parity audit. |
| **`/audit-hardware-quality`** | [`audit-hardware-quality.md`](../.agents/workflows/audit-hardware-quality.md) | Full-workspace hardware architectural audit (bus topology, Agnus DMA mastership, passive latching, CCK). |
| **`/test-runner`** | [`test-runner.md`](../.agents/workflows/test-runner.md) | Standardized test suite execution, `.test_results/` snapshot rotation, and automated regression diffing. |
| **`/integration-test-sprint`** | [`integration-test-sprint.md`](../.agents/workflows/integration-test-sprint.md) | 4-iteration cascading verification sweep, failure clustering, and 2–3 attempt time-boxing. |
| **`/synthesize-test-fixes`** | [`synthesize-test-fixes.md`](../.agents/workflows/synthesize-test-fixes.md) | Post-facto git diff audit, 3-column diagnostic matrix construction, and root-cause consolidation into upstream substrate crates. |
| **`/graphify`** | [`graphify.md`](../.agents/workflows/graphify.md) | AST knowledge graph queries, call hierarchy extraction, and scoped subtree re-indexing. |
| **`/compact-diary`** | [`compact-diary.md`](../.agents/workflows/compact-diary.md) | Milestone synthesis and compaction of historical chronological entries in DIARY.md. |
| **`/sync-design-docs`** | [`sync-design-docs.md`](../.agents/workflows/sync-design-docs.md) | Audit and synchronize Obsidian design specifications with active Rust code commits and bump checkpoints. |
| **`/roadmap-maintenance`** | [`roadmap-maintenance.md`](../.agents/workflows/roadmap-maintenance.md) | Prune completed steps from ROADMAP.md (zero retention) and apply substrate-first causal renumbering. |
| **`/index-amiga-rag`** | [`index-amiga-rag.md`](../.agents/workflows/index-amiga-rag.md) | Incremental vector re-indexing of Amiga hardware manuals and Obsidian notes into local Qdrant database. |

---

## 5. Native Subagents (`.agents/agents/`)

Subagents run in their own **isolated context windows**, shielding the main architect
session from token-heavy bulk processing (OCR floods, multi-page markdown dumps, graph
traversals). Each subagent is invoked by the parent session and returns only a compact,
high-signal verdict or diff.

| Subagent | File | Context Profile | Primary Responsibility |
| :--- | :--- | :--- | :--- |
| **`cpu_verifier`** | [`cpu_verifier/agent.md`](../.agents/agents/cpu_verifier/agent.md) | 🔬 Precision / Low volume | Tom Harte single-step silicon test execution, cycle-exact ALU/CCR verification, and timing regression isolation. |
| **`code_reviewer`** | [`code_reviewer/agent.md`](../.agents/agents/code_reviewer/agent.md) | 🔍 Adversarial / Medium volume | 18-point pre-commit and architectural compliance audit against AGENTS.md rules, file size limits, and inlining policy. |
| **`vision_analyst`** | [`vision_analyst/agent.md`](../.agents/agents/vision_analyst/agent.md) | 🖼️ Multimodal / Medium volume | Circuit schematic interpretation, timing diagram analysis, and `egui` visual layout debugging via multimodal vision. |
| **`doc_curator`** | [`doc_curator/agent.md`](../.agents/agents/doc_curator/agent.md) | 📐 Structural / Low volume | Semantic parity between `Obsidian/Amiga/Design/` specs and Rust code, vault graph integrity, ROADMAP.md pruning, DIARY.md compaction. |
| **`doc_ingestor`** | [`doc_ingestor/agent.md`](../.agents/agents/doc_ingestor/agent.md) | 📦 Heavy data / Isolated | PDF/HTML → Markdown conversion of reference manuals, circuit schematic vision sidecars, and Qdrant vector reindexing. |
| **`tech_writer`** | [`tech_writer/agent.md`](../.agents/agents/tech_writer/agent.md) | ✍️ Narrative / Medium volume | Long-form retrospective essays, engineering devlogs, and methodology documents under `docs/` using practitioner voice and 6-layer Inverted Pyramid. |

### Documentation Subagent Split: Why Two Agents?

The `doc_curator` / `doc_ingestor` split is a deliberate context isolation boundary:

```
Main Session (Lead Architect)
    │
    ├─▶ doc_curator   ← Semantic spec sync, vault links, ROADMAP, DIARY
    │      Context: structural & precision-oriented; low raw text volume
    │
    ├─▶ doc_ingestor  ← PDF/HTML conversion, schematic sidecars, RAG reindex
    │      Context: token-heavy bulk processing (OCR, multi-page markdown)
    │               isolated entirely in its own window
    │
    └─▶ tech_writer   ← Retrospective essays, devlogs, methodology docs
           Context: narrative prose; mines DIARY.md for architectural history
```

**Rule of thumb:**
- Reasoning about *existing* architectural specs → `doc_curator`.
- *Transforming raw external data* (manuals, PDFs, HTML archives) → `doc_ingestor`.
- *Narrating architectural decisions* as long-form prose for human readers → `tech_writer`.

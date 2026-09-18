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

Operational procedures and recipes are modularized under [`.agents/skills/`](../.agents/skills/) across three primary engineering domains:

### A. CPU & Hardware Emulation
- [`add-m68k-instruction`](../.agents/skills/add-m68k-instruction/SKILL.md): Step-by-step recipe for implementing new M68000 instructions (decoding, CCK micro-steps, inlined CCR flags, and single-step test validation).
- [`m68k-singlestep-test`](../.agents/skills/m68k-singlestep-test/SKILL.md): Diagnostic runbook for running, isolating, and troubleshooting Tom Harte physical silicon single-step hardware test failures.

### B. Quality Assurance & Code Hygiene
- [`code-review`](../.agents/skills/code-review/SKILL.md): Comprehensive 14-point audit checklist for code quality, file sizes ($\le 800$ lines), inlining, zero runtime panics, and spec compliance.
- [`test-runner`](../.agents/skills/test-runner/SKILL.md): Standardized test execution across all 4 tiers, `.test_results/` snapshot management, and automated differential regression telemetry.
- [`integration-test-sprint`](../.agents/skills/integration-test-sprint/SKILL.md): 4-Iteration Cascading Verification Protocol runbook for multi-chip integration test suites, failure clustering, and 2–3 attempt limits.
- [`synthesize-test-fixes`](../.agents/skills/synthesize-test-fixes/SKILL.md): Post-facto root-cause consolidation using the 3-Column Diagnostic Matrix (`[Symptom] | [Location] | [Mechanism]`) to replace scattered local workarounds with a unified upstream hardware model.
- [`audit-code-quality`](../.agents/skills/audit-code-quality/SKILL.md): Comprehensive on-demand code quality auditor and pruning playbook covering dead code, test-only zombies, minimum visibility leaks, and SRP cohesion across workspace crates.
- [`egui-vision-debugger`](../.agents/skills/egui-vision-debugger/SKILL.md): Headless visual inspection and autonomous self-healing skill for the `egui` frontend using `gui-inspector` (`egui_kittest` + `wgpu`) and Agent Multimodal Vision to diagnose layout squishing, splitter contention, and focus lifecycles.

### C. Architecture, Knowledge & Documentation
- [`obsidian-vault-linking`](../.agents/skills/obsidian-vault-linking/SKILL.md): Enforces Line 1 YAML properties (`tags: [spec, ...]`), inverted pyramid structure, dual-layer linking (inline contextual + bottom structural references), and zero broken links across [Obsidian/Amiga/Design/](../Obsidian/Amiga/Design/).
- [`compact-diary`](../.agents/skills/compact-diary/SKILL.md): Milestone compaction procedure synthesizing older chronological log entries in [DIARY.md](../DIARY.md) into concise architectural digests while preserving key evolutionary rationale and verified results.
- [`pdf-to-markdown`](../.agents/skills/pdf-to-markdown/SKILL.md): High-fidelity document conversion toolchain (PyMuPDF chapter splitting, figure cropping, SVG vectorization, and table stitching) for technical reference manuals.
- [`index-amiga-rag`](../.agents/skills/index-amiga-rag/SKILL.md): Operational procedure for Qdrant vector reindexing, status verification, and offline visual diagram sidecar maintenance (`<image>.txt`).
- [`author-methodology-doc`](../.agents/skills/author-methodology-doc/SKILL.md): Author, audit, or restructure narrative articles, methodology documents, essays, and retrospective devlogs (e.g. `docs/how_this_emulator_was_written.md`) using the 6-layer Inverted Pyramid hierarchy.
- [`html-to-markdown`](../.agents/skills/html-to-markdown/SKILL.md): Standardized toolchain for converting legacy Word HTML, vintage web documentation, and technical HTML articles into clean, publication-grade Obsidian Markdown with asset extraction, layout unnesting, and anchor link validation.
- [`graphify`](../.agents/skills/graphify/SKILL.md): Persistent code knowledge graph navigation, call hierarchy tracing, and scoped subtree updates (`crates/` vs `ref_src/`).

---

## 4. Interactive Slash Command Workflows (`.agents/workflows/`)

Developers trigger high-level orchestration directly in the IDE chat UI using slash commands:

| Slash Command | Workflow File | Primary Purpose |
| :--- | :--- | :--- |
| **`/code-review`** | [`code-review.md`](../.agents/workflows/code-review.md) | Comprehensive 14-point pre-commit and milestone architectural compliance audit. |
| **`/test-runner`** | [`test-runner.md`](../.agents/workflows/test-runner.md) | Standardized test suite execution, `.test_results/` snapshot rotation, and automated regression diffing. |
| **`/integration-test-sprint`** | [`integration-test-sprint.md`](../.agents/workflows/integration-test-sprint.md) | 4-iteration cascading verification sweep, failure clustering, and 2–3 attempt time-boxing. |
| **`/synthesize-test-fixes`** | [`synthesize-test-fixes.md`](../.agents/workflows/synthesize-test-fixes.md) | Post-facto git diff audit, 3-column diagnostic matrix construction, and root-cause consolidation into upstream substrate crates. |

# Autonomous AI Agent Engineering & Pair-Programming Guide

This repository is engineered from the ground up for autonomous AI agent pair-programming with strict architectural guardrails.

---

## 1. Architectural Guardrails & Invariants

All automated modifications and agent sessions must strictly adhere to [`AGENTS.md`](../AGENTS.md) and modularized rules under [`.agents/rules/`](../.agents/rules/):

- **Language Policy (`language-policy.md`):** Strict English for all code, identifiers, comments, documentation, and commits.
- **Audio Voice Transcription (`audio-transcription.md`):** Transcribe spoken user voice recordings at the top of responses before proceeding in English.
- **Strict Path Privacy (`no-external-paths.md`):** Zero external host paths in committed files; use generic placeholders.
- **Zero Host Panics (`performance-and-readability.md`):** No `.unwrap()` or `.expect()` in runtime emulation hot paths.
- **Hardware Efficiency & Readability:** Zero custom macros (`macro_rules!`), zero const-generic instruction handlers, contiguous memory layouts, and zero heap allocations in execution loops.
- **Attractor Discipline (`attractor-discipline.md`):** Zero synthetic academic jargon, theatrical testing metaphors, or heading slogans.
- **Amiga RAG Knowledge Base (`amiga-rag.md`):** Mandatory pre-task conceptual retrieval and automated reindexing on documentation changes.
- **Graphify AST Knowledge Graph (`graphify.md`):** Consult the code knowledge graph for AST queries and adhere to scoped subtree re-indexing (`crates/` vs `ref_src/`).

---

## 2. Knowledge Retrieval: RAG & Graphify

### A. Domain Hardware Knowledge: Local Vector RAG (`amiga-rag`)
- **Knowledge Base Scope:** Connects to local Qdrant database (`http://localhost:6333`, collection: `amiga`), indexing Commodore Hardware Reference Manuals, M68000 PRMs, Guru book, and design specs under `Obsidian/Amiga/`.
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
- **Scoped Subtree Re-indexing Rule ([`graphify.md`](../.agents/rules/graphify.md)):**
  - To prevent slow full-repository re-crawls during localized edits, both codebases are indexed and updated independently:
    - On modifications within active crates: `graphify update crates/`
    - On modifications within reference sources: `graphify update ref_src/`
  - Full-repository indexing (`graphify update .`) is reserved for initial setup via `.\tools\bootstrap.ps1 -Graph` or major cross-cutting refactors.
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
- [`attractor-discipline`](../.agents/skills/attractor-discipline/SKILL.md): Automated linter script and guidelines for detecting and cleaning quarantined linguistic attractors, theatrical metaphors, and heading slogans.
- [`prune-dead-code`](../.agents/skills/prune-dead-code/SKILL.md): Systematic procedure for identifying and safely eliminating dead code, unused functions, obsolete constants, and unreferenced crate exports upon milestone completion.
- [`egui-vision-debugger`](../.agents/skills/egui-vision-debugger/SKILL.md): Headless visual inspection and autonomous self-healing skill for the `egui` frontend using `gui-inspector` (`egui_kittest` + `wgpu`) and Agent Multimodal Vision to diagnose layout squishing, splitter contention, and focus lifecycles.

### C. Architecture, Knowledge & Documentation
- [`obsidian-vault-linking`](../.agents/skills/obsidian-vault-linking/SKILL.md): Enforces Line 1 YAML properties (`tags: [spec, ...]`), inverted pyramid structure, dual-layer linking (inline contextual + bottom structural references), and zero broken links across [Obsidian/Amiga/Design/](../Obsidian/Amiga/Design/).
- [`compact-diary`](../.agents/skills/compact-diary/SKILL.md): Milestone compaction procedure synthesizing older chronological log entries in [DIARY.md](../DIARY.md) into concise architectural digests while preserving key evolutionary rationale and verified results.
- [`pdf-to-markdown`](../.agents/skills/pdf-to-markdown/SKILL.md): High-fidelity document conversion toolchain (PyMuPDF chapter splitting, figure cropping, SVG vectorization, and table stitching) for technical reference manuals.
- [`amigaguide-to-markdown`](../.agents/skills/amigaguide-to-markdown/SKILL.md): Hypertext conversion pipeline for Commodore AmigaGuide documents (`.guide`) into linked Markdown files.
- [`index-amiga-rag`](../.agents/skills/index-amiga-rag/SKILL.md): Operational procedure for Qdrant vector reindexing, status verification, and offline visual diagram sidecar maintenance (`<image>.txt`).
- [`graphify`](../.agents/skills/graphify/SKILL.md): Persistent code knowledge graph navigation, call hierarchy tracing, and scoped subtree updates (`crates/` vs `ref_src/`).

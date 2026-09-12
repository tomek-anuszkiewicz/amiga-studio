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
- **Graphify AST Knowledge Graph (`graphify.md`):** Consult the code knowledge graph for AST queries and adhere to scoped subtree re-indexing (`crates/` vs `ref_src/`).

---

## 2. Knowledge Retrieval: RAG & Graphify

### A. Domain Hardware Knowledge: Local Vector RAG (`amiga-rag`)
- Connects to local Qdrant database (`http://localhost:6333`, collection: `amiga`).
- Indexes official Commodore Hardware Reference Manuals, 68000 PRMs, and Guru book under `Obsidian/Amiga/Reference/`, alongside design specs under `Obsidian/Amiga/Design/`.
- Query via tool: `rag_search(query="<topic>", sources=["amiga", "obsidian"])`.
- Run indexing via CLI: `.\tools\rag\bin\amiga_rag.ps1 "Obsidian/Amiga" --source amiga`.

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

- [`add-m68k-instruction`](../.agents/skills/add-m68k-instruction/SKILL.md): Step-by-step recipe for implementing new M68000 instructions (decoding, CCK micro-steps, inlined CCR flags, and single-step test validation).
- [`m68k-singlestep-test`](../.agents/skills/m68k-singlestep-test/SKILL.md): Diagnostic runbook for running and troubleshooting Tom Harte single-step hardware test failures.
- [`attractor-discipline`](../.agents/skills/attractor-discipline/SKILL.md): Automated linter for detecting and cleaning quarantined linguistic attractors.
- [`code-review`](../.agents/skills/code-review/SKILL.md): Comprehensive 14-point audit for code quality, file sizes, inlining, and spec compliance.

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

---

## 2. Knowledge Retrieval: RAG & Graphify

### A. Domain Hardware Knowledge: Local Vector RAG (`amiga-rag`)
- Connects to local Qdrant database (`http://localhost:6333`, collection: `amiga`).
- Indexes official Commodore Hardware Reference Manuals, 68000 PRMs, and Guru book under `Obsidian/Amiga/Reference/`, alongside design specs under `Obsidian/Amiga/Design/`.
- Query via tool: `rag_search(query="<topic>", sources=["amiga", "obsidian"])`.
- Run indexing via CLI: `.\tools\rag\bin\amiga_rag.ps1 "Obsidian/Amiga" --source amiga`.

### B. Code Structure & Relationships: AST Knowledge Graph (`graphify`)
- Use `graphify` (`graphify query`, `graphify path`, `graphify explain`) to analyze call hierarchies, dependencies, and type relations across crates.

---

## 3. Specialized Agent Skills (`.agents/skills/`)

- [`add-m68k-instruction`](../.agents/skills/add-m68k-instruction/SKILL.md): Step-by-step recipe for implementing new M68000 instructions (decoding, CCK micro-steps, inlined CCR flags, and single-step test validation).
- [`m68k-singlestep-test`](../.agents/skills/m68k-singlestep-test/SKILL.md): Diagnostic runbook for running and troubleshooting Tom Harte single-step hardware test failures.
- [`attractor-discipline`](../.agents/skills/attractor-discipline/SKILL.md): Automated linter for detecting and cleaning quarantined linguistic attractors.
- [`code-review`](../.agents/skills/code-review/SKILL.md): Comprehensive 14-point audit for code quality, file sizes, inlining, and spec compliance.

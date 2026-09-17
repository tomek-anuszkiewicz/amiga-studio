# Developer Guide & Tooling Index

This document serves as the primary technical entry point for building, testing, verifying, and contributing to the Amiga 500 emulator.

---

## 1. Zero-Setup Build & Execution

A freshly cloned repository is **100% self-contained for compilation and execution** using the standard stable Rust toolchain ([rustup.rs](https://rustup.rs)). No bootstrapping, external downloads, or database services are required to build and run the emulator:

```powershell
# Build entire workspace (debug profile):
cargo build

# Build optimized native release binary for GUI:
cargo build --release -p gui
```

---

## 2. Optional Bootstrapping (`tools/bootstrap/bootstrap.ps1`)

Bootstrapping is **strictly optional** and only needed for specialized development tasks:

| Mode | Switch | When Needed | What It Provisions |
| :--- | :--- | :--- | :--- |
| **Verification Testbed** | `-Sources` | Running exhaustive single-step M68000 suites and DMA contention stress tests | Provisions Tom Harte physical silicon test vectors (auto-decompressing `.gz`/`.zip` archives in `ref_src/SingleStepTests-680x0/`, 124 suites), verifies vAmiga/vAmigaTS reference suites, and diagnostic disks |
| **Code Knowledge Graph** | `-Graphify` | Codebase structural navigation, call hierarchy, and symbol dependency analysis | AST-level code knowledge graph (`graphify-out/`), mapping crates, structs, functions, and cross-module relationships |
| **Documentation & RAG** | `-Rag` | AI agent pair-programming, hardware research, architecture design | Local Qdrant vector database (`http://localhost:6333`), indexing Commodore HRM, 68000 PRMs, technical specs, and design specs |
| **Reference Scans** | `-Documentation` | External reference scans and manual archives | Provisions raw reference manuals and PDF scans into `temp/` |
| **Full Setup** | `-All` | Complete initial development setup | Provisions all primary components (sources -> Graphify AST -> RAG documentation) |

### Bootstrapper Commands

```powershell
# Hardware test vectors verification setup:
.\tools\bootstrap\bootstrap.ps1 -Sources

# Code AST knowledge graph setup:
.\tools\bootstrap\bootstrap.ps1 -Graphify

# Documentation & AI pair-programming setup:
.\tools\bootstrap\bootstrap.ps1 -Rag

# External reference manuals and scans:
.\tools\bootstrap\bootstrap.ps1 -Documentation

# Complete setup (sources -> Graphify AST -> RAG docs):
.\tools\bootstrap\bootstrap.ps1 -All
```

---

## 3. Developer Documentation & Verification Subsystems

- [**Test Suite & Verification Framework**](testing.md): Physical hardware single-step test options (`SINGLESTEP_FULL`, `SINGLESTEP_LIMIT`), Cartesian DMA contention math ($2^k \times 2^M$), vAmigaTS subsystem integration, and CLI regression diagnostics.
- [**External Reference Sources & Testbeds Guide**](reference_sources.md): Catalog of external repositories (Tom Harte SingleStepTests, vAmiga, vAmigaTS, AmigaTestKit), pinned versions, and automated provisioning.
- [**Technical Reference Documentation & AI RAG Guide**](reference_documentation.md): Ingested hardware reference manuals, automated reference bootstrapper, raw document conversion toolchain, and local Qdrant RAG search.

---

## 4. Knowledge Retrieval: RAG & Graphify

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
  - After modifications to code in `crates/` or `ref_src/`, run `graphify update .` from the repository root (or via `.\tools\bootstrap\bootstrap.ps1 -Graphify`).
  - This keeps a single unified knowledge graph in `graphify-out/` connecting active emulator crates and reference implementations.
- **Query Tools:**
  - `graphify query "<question>"`: Query symbol dependencies, call hierarchies, and architectural boundaries.
  - `graphify path "<A>" "<B>"`: Trace the shortest dependency or call path between two types or functions.
  - `graphify explain "<concept>"`: Extract a focused subgraph explaining a subsystem or module.


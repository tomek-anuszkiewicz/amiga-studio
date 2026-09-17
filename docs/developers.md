# Developer Guide & Tooling Index

This document serves as the primary technical entry point for building, testing, verifying, and contributing to the Amiga 500 emulator. It consolidates foundational architecture, build workflows, optional bootstrapping, external reference sources, ingested documentation, and knowledge retrieval tools.

> [!IMPORTANT]
> **Foundational Architecture:** Before modifying or extending emulator subsystems, review the [**Core Architecture & Hardware Execution Model**](architecture.md). It formalizes the cycle-exact Color Clock phases (`CCK1`/`CCK2`), shared bus contention, Gary arbitration, Big-Endian invariance, and decoupled subsystem ownership. Comprehensive silicon specifications, register allocations, and circuit schematics reside in [`Obsidian/Amiga/Design/`](../Obsidian/Amiga/Design/).

---

## Table of Contents

- [1. Build & Run](#1-build-run)
  - [Standalone Native GUI (Release)](#standalone-native-gui-release)
  - [WebAssembly Browser Canvas (Trunk)](#webassembly-browser-canvas-trunk)
- [2. External Reference Sources & Verification Testbeds](#2-external-reference-sources-verification-testbeds)
  - [Pinned Upstream Sources Summary](#pinned-upstream-sources-summary)
- [3. Documentation Architecture & Reference Literature](#3-documentation-architecture-reference-literature)
  - [Why Documentation is Critical (Human, AI & RAG)](#why-documentation-is-critical-human-ai-rag)
  - [External Reference Documentation (Bootstrapped / Git-Ignored)](#external-reference-documentation-bootstrapped-git-ignored)
- [4. Bootstrapping & External Reference Provisioning (`tools/bootstrap/bootstrap.ps1`)](#4-bootstrapping-overview-toolsbootstrapbootstrapps1)
  - [Complete Development Setup (`-All`)](#full-development-setup-all)
  - [Verification Testbeds Provisioning (`-Sources`)](#sources-provisioning-sources)
  - [Archival Reference Documentation Provisioning (`-Documentation`)](#archival-documentation-provisioning-documentation)
  - [Domain Hardware RAG Setup (`-Rag`)](#domain-hardware-rag-setup-rag)
  - [Code Knowledge Graph Setup (`-Graphify`)](#code-knowledge-graph-setup-graphify)
- [5. Domain Hardware Knowledge: Local Vector RAG (`amiga-rag`)](#5-domain-hardware-knowledge-local-vector-rag-amiga-rag)
- [6. Code Structure & Relationships: Graphify](#6-code-structure-relationships-graphify)
- [7. Test Suite & Verification Framework](#7-test-suite-verification-framework)
  - [A. M68000 SingleStepTests (Silicon Verification)](#a-m68000-singlesteptests-physical-hardware-silicon-verification)
  - [B. Cartesian DMA Contention Verification](#b-cartesian-dma-contention-verification)
  - [C. Automated Architecture Rules Compliance](#c-automated-architecture-rules-compliance)
  - [D. CLI Diagnostic Tools & Regression Tracking](#d-cli-diagnostic-tools-regression-tracking)
  - [E. vAmigaTS Verification Framework & Golden Viewport Testing](#e-vamigats-verification-framework-golden-viewport-testing)

---

<a id="1-build-run"></a><a id="1-build--run"></a><a id="1-zero-setup-build-execution"></a>
## 1. Build & Run

A freshly cloned repository is **100% self-contained for compilation and execution** using the standard stable Rust toolchain ([rustup.rs](https://rustup.rs)). No bootstrapping, external downloads, or database services are required to run the emulator.

<a id="standalone-native-gui-release"></a>
### Standalone Native GUI (Release)

Run the optimized native desktop GUI (Developer Studio & Interactive Debugger):
```powershell
cargo run --release -p gui
```

> To build the standalone release binary without running it immediately:
> ```powershell
> cargo build --release -p gui
> ```

<a id="webassembly-browser-canvas-trunk"></a>
### WebAssembly Browser Canvas (Trunk)

Run the emulator directly inside a web browser using WebAssembly and [Trunk](https://trunkrs.dev):
```powershell
# Install Trunk (first-time only):
cargo install --locked trunk

# Serve and open the WebAssembly canvas in your default browser:
trunk serve crates/gui/index.html --open
```

---

<a id="2-external-reference-sources-verification-testbeds"></a><a id="2-external-reference-sources--verification-testbeds"></a><a id="3-external-reference-sources-verification-testbeds"></a>
## 2. External Reference Sources & Verification Testbeds

The emulator core does not rely on guesswork or high-level approximations; it validates execution against authoritative external test suites, physical silicon captures, and clean-room reference implementations:

- **M68000 CPU Testing (Tom Harte SingleStepTests):** Used for exhaustive, cycle-exact verification of the Motorola 68000 processor core. Evaluates instruction execution, prefetch queue progression, and Condition Code Register (CCR) flag calculations against ~1,000,000 test vectors captured directly from physical 68000 silicon pins.
- **Whole-Machine & Chipset Verification (vAmiga Test Suite - vAmigaTS):** Used to test the emulator against golden Amiga 500 hardware behavior. Contains 2,077 test cases validating Copper timing, Blitter DMA, Denise display window clipping, and Paula audio by comparing rendered frames against golden 716×285 24-bit RGB viewport captures recorded from real Amigas.
- **Clean-Room C++ Reference (vAmiga):** Used as an architectural and algorithmic reference implementation for custom chipset timing, bus arbitration, and register interactions.
- **Hardware Diagnostics (Amiga Test Kit):** Bootable diagnostic floppy disk image used for end-to-end system loop verification, memory testing, and peripheral validation.

> [!NOTE]
> **Graphify Code Knowledge Graph Integration:** Downloaded reference source files (specifically `ref_src/vAmiga/` and test harnesses) are parsed and indexed directly into the unified **Graphify graph database** (`graphify-out/`) alongside active Rust crates (`crates/`). This allows developers and AI agents to query cross-codebase symbol definitions, trace call hierarchies, and compare Rust implementations against reference C++ algorithms via `graphify query` (see [Chapter 6: Code Structure & Relationships: Graphify](#6-code-structure-relationships-graphify)).

Because these test assets comprise multi-gigabyte datasets (~6.5 GB uncompressed), they reside outside Git history in `ref_src/` and `tools/`.

<a id="pinned-upstream-sources-summary"></a>
### Pinned Upstream Sources Summary

| Repository / Asset | Local Path | Upstream Repository & URL | Pinned Version / Release |
| :--- | :--- | :--- | :--- |
| **Tom Harte SingleStepTests** | `ref_src/SingleStepTests-680x0/` | [SingleStepTests/680x0](https://github.com/SingleStepTests/680x0) | **Format v1** (`68000/v1/`), 124 opcode suites |
| **vAmiga C++ Emulator** | `ref_src/vAmiga/` | [dirkwhoffmann/vAmiga](https://github.com/dirkwhoffmann/vAmiga) | **Release v4.5** |
| **vAmiga Test Suite (vAmigaTS)** | `ref_src/vAmigaTS/` | [dirkwhoffmann/vAmigaTS](https://github.com/dirkwhoffmann/vAmigaTS) | **`master`** (2,077 test directories) |
| **Amiga Test Kit** | `tools/AmigaTestKit/AmigaTestKit.adf` | [keirf/amiga-stuff](https://github.com/keirf/amiga-stuff) | **Release v1.20+** (`AmigaTestKit.adf`) |

> [!TIP]
> Information and automated scripts for downloading, decompressing, and provisioning these reference test sources are provided in **[Chapter 4: Bootstrapping & External Reference Provisioning](#sources-provisioning-sources)** (`.\tools\bootstrap\bootstrap.ps1 -Sources`).

---

<a id="3-documentation-architecture-reference-literature"></a><a id="3-documentation-architecture--reference-literature"></a><a id="3-ingested-reference-documentation-in-repository"></a><a id="3-ingested-reference-documentation--in-repository"></a><a id="3-technical-reference-documentation-literature"></a><a id="3-technical-reference-documentation--literature"></a>
## 3. Documentation Architecture & Reference Literature

The repository maintains an authoritative, high-fidelity documentation library under `Obsidian/Amiga/`. To optimize repository clone sizes and respect licensing boundaries, documentation is strictly partitioned into two categories:

<a id="why-documentation-is-critical-human-ai-rag"></a><a id="why-ingested-documentation-is-critical"></a>
### Why Documentation is Critical (Human, AI & RAG)

1. **Direct Human Exploration & Research:**
   - Provides instant, friction-free access to Commodore and Motorola engineering specifications directly inside your IDE or Obsidian vault.
   - Eliminates the need to search through 400+ page unindexed PDF scans or hunt down defunct retro-computing websites.

2. **Autonomous AI Pair-Programming (Direct & via RAG):**
   - **Direct Consumption:** AI coding agents inspect structured Markdown specifications, circuit diagrams, and register tables to implement cycle-exact micro-operations and verify ALU behaviors.
   - **Local Vector RAG Retrieval (`amiga-rag`):** The entire reference library (alongside `Obsidian/Amiga/Design/` architectural specs) is indexed in Qdrant. Autonomous agents perform pre-task conceptual queries before touching code, ensuring zero silent divergence from hardware silicon.
   - **Interactive Consultation & Change Review:** Developers can query the RAG system directly through CLI or FastMCP tools (`rag_search`) to clarify complex hardware quirks, verify register behavior, or evaluate architectural changes before editing production code.

<a id="in-repository-specifications-committed-to-git"></a><a id="external-reference-documentation-bootstrapped-git-ignored"></a><a id="ingested-reference-documents-available-in-repository"></a>
### External Reference Documentation (Bootstrapped / Git-Ignored)

Due to copyright preservation and multi-gigabyte dataset exclusion (`.gitignore`), the primary Commodore and Motorola reference manuals are **excluded from Git tracking**. They are converted into structured Markdown format and must be downloaded or linked via bootstrapping:

1. **Hardware Reference Manual (Addison-Wesley 2nd Edition, 1989)**
   - **Location:** `Obsidian/Amiga/Reference/Hardware Reference Manual/`
   - **Scope:** Primary reference for Amiga custom chipsets (OCS): Agnus (Copper, Blitter), Denise (Bitplanes, Sprites, Color palette), Paula (Audio DMA, Floppy disk controller), interrupt priority routing, and memory map.
   - **Provenance:** Based strictly on the Addison-Wesley 2nd Edition (1989) typeset on Commodore Amiga 2500/UX (AMIX), covering pure A500 OCS hardware without ECS contamination.

2. **A500 A2000 Technical Reference Manual (Commodore-Amiga OEM, 1987)**
   - **Location:** `Obsidian/Amiga/Reference/A500 A2000 Technical Reference Manual/`
   - **Scope:** Official Commodore OEM engineering manual covering bus timing, Gary gate array logic, system motherboard schematics, expansion bus (Zorro), and bridgeboard signals.

3. **M68000 Programmer's Reference Manual (Motorola Rev 1, 1992)**
   - **Location:** `Obsidian/Amiga/Reference/68000 Programmer's Reference Manual/`
   - **Scope:** Authoritative instruction set definitions (68000 core), addressing modes, Condition Code Register (CCR) flag calculations, and execution cycle charts.

4. **M68000 User's Manual (Motorola Rev 8, 1993)**
   - **Location:** `Obsidian/Amiga/Reference/68000 User's Manual/`
   - **Scope:** Cycle-by-cycle bus timing diagrams, bus state phases ($S0$–$S7$), read/write cycles, wait states, bus arbitration signals (`BR`, `BG`, `BGACK`), pinouts, and electrical characteristics.

5. **Instruction Prefetch on the Motorola 68000 Processor (Jorge Cwik / Pasti, 2005)**
   - **Location:** `Obsidian/Amiga/Reference/Instruction Prefetch on the Motorola 68000 Processor.md`
   - **Scope:** Authoritative microarchitectural prefetch queue study (Version 1.3). Establishes the 68000 two-stage `IR` (Instruction Register) and `IRC` (Instruction Register Capture) pipeline model and timing interactions.

6. **DIVU & DIVS Cycle-Accurate Timing Analysis (Jorge Cwik / Pasti):**
   - **Location:** `Obsidian/Amiga/Reference/Motorola 68000 DIVU & DIVS Cycle-Accurate Timing Analysis.md`
   - **Scope:** Cycle-exact algorithmic analysis of 68000 non-restoring integer division. Formulates precise cycle calculations verified against physical silicon test vectors.

7. **Undocumented Features of OCS, ECS and AGA Chipsets (Kuba Winnicki / Achtung! Amiga, 2002)**
   - **Location:** `Obsidian/Amiga/Reference/Undocumented features of OCS, ECS and AGA chipsets.md`
   - **Scope:** 16-chapter investigation into silicon quirks: Copper hazards, sprite demultiplexing, DMA slot arbitration, UHRES display modes, and video beam timing anomalies.

> [!TIP]
> Information and automated scripts for downloading, bootstrapping, and converting these external reference materials into Markdown are provided in **[Chapter 4: Archival Reference Documentation Provisioning (`-Documentation`)](#archival-documentation-provisioning-documentation)** (`.\tools\bootstrap\bootstrap.ps1 -Documentation`).

---

<a id="4-bootstrapping-overview-toolsbootstrapbootstrapps1"></a><a id="4-bootstrapping-overview--toolsbootstrapbootstrapps1"></a><a id="5-bootstrapping-overview-toolsbootstrapbootstrapps1"></a><a id="5-bootstrapping-overview--toolsbootstrapbootstrapps1"></a><a id="4-external-reference-downloads-provisioning"></a><a id="4-external-reference-downloads--provisioning"></a><a id="4-archival-documentation-reference-downloads"></a><a id="4-archival-documentation--reference-downloads"></a><a id="2-optional-bootstrapping-overview"></a><a id="bootstrapping-overview"></a>
## 4. Bootstrapping & External Reference Provisioning (`tools/bootstrap/bootstrap.ps1`)

Bootstrapping is **strictly optional**. A freshly cloned repository compiles and runs the standalone desktop GUI or WebAssembly canvas out of the box using only the stable Rust toolchain.

However, specialized development and verification workflows—such as running exhaustive 124-suite M68000 silicon tests, validating against golden vAmigaTS visual viewports, querying the offline Qdrant RAG vector database, or exploring cross-codebase call graphs—require external reference assets, technical literature, or tooling services.

The centralized bootstrapper (`tools/bootstrap/bootstrap.ps1`) orchestrates all provisioning workflows.

<a id="full-development-setup-all"></a><a id="bootstrapper-commands"></a>
### Complete Development Setup (`-All`)

To provision every external reference asset, knowledge base, and tooling index in a single run, use the `-All` switch:

```powershell
.\tools\bootstrap\bootstrap.ps1 -All
```

| Mode | Switch | When Needed | What It Provisions |
| :--- | :--- | :--- | :--- |
| **Full Setup** | `-All` | Complete initial development setup | Executes all 4 phases sequentially in causal order: **Sources** $\to$ **Documentation** $\to$ **RAG** $\to$ **Graphify** |
| **Verification Testbeds** | `-Sources` | Running exhaustive single-step M68000 suites and DMA contention stress tests | Downloads and decompresses external test suites, M68000 silicon vectors (`SingleStepTests-680x0`), vAmiga/vAmigaTS reference testbeds, and the Amiga Test Kit ADF |
| **External Reference Scans** | `-Documentation` | External reference scans and manual archives | Downloads original archival PDF scans and OEM technical reference manuals (HRM, TRM, PRM, UM, and technical articles) |
| **Documentation & RAG** | `-Rag` | AI agent pair-programming, hardware research, architecture design | Initializes local Qdrant vector database (`http://localhost:6333`) and indexes reference manuals (`Obsidian/Amiga/Reference/`) and design specs (`Obsidian/Amiga/Design/`) via offline FastEmbed embeddings |
| **Code Knowledge Graph** | `-Graphify` | Codebase structural navigation, call hierarchy, and symbol dependency analysis | Performs AST analysis across active Rust crates (`crates/`) and reference C++ code (`ref_src/vAmiga`), generating the unified symbol dependency and call-hierarchy graph in `graphify-out/` |

---

### Granular Provisioning Workflows

Each provisioning phase can also be executed independently on demand:

<a id="automated-provisioning-toolsbootstrapbootstrap_sourcesps1"></a><a id="sources-provisioning-sources"></a><a id="reference-sources-lifecycle-steps"></a><a id="automation-lifecycle-steps"></a>
#### 1. Verification Testbeds Provisioning (`-Sources`)

Automates downloading, unpacking, decompressing, and validating external test assets and reference code:

```powershell
.\tools\bootstrap\bootstrap.ps1 -Sources
```

##### Automation Lifecycle Steps

When external test sources are provisioned via `.\tools\bootstrap\bootstrap.ps1 -Sources`, the automated bootstrapper executes the following lifecycle steps:

1. **Zip Archive Expansion:** Scans `ref_src/SingleStepTests-680x0/` for `.zip` archives and unpacks them into place.
2. **Gzip Decompression:** Scans for `.json.gz` or `.gz` compressed test archives and decompresses them into native `.json` files using .NET `GZipStream` (zero external dependencies).
3. **Directory Canonicalization:** Migrates any loose `.json` test suites from `68000/` into the canonical `68000/v1/` directory.
4. **Presence & Completeness Validation:** Verifies that `SingleStepTests-680x0` contains all 124 test suites, checks for `tools/AmigaTestKit/AmigaTestKit.adf`, and validates `ref_src/vAmiga` and `ref_src/vAmigaTS`.

> [!TIP]
> **Fine-Grained Execution:** When you need direct execution, testing, or pipeline debugging without invoking the coordinator wrapper, invoke the dedicated sources bootstrapper directly:
> ```powershell
> .\tools\bootstrap\bootstrap_sources.ps1
> ```

<a id="bootstrapping-raw-archival-sources-toolsbootstrapbootstrap_documentationps1"></a><a id="archival-documentation-provisioning-documentation"></a><a id="upstream-archival-reference-documents"></a><a id="multi-source-fallback-matrix-error-resilience"></a><a id="multi-source-fallback-matrix--error-resilience"></a>
#### 2. Archival Reference Documentation Provisioning (`-Documentation`)

Provisions original PDF scans, OEM technical manuals, and archival web articles for developers inspecting raw schematics or re-running OCR pipelines:

```powershell
.\tools\bootstrap\bootstrap.ps1 -Documentation
```

<a id="multi-page-web-crawling-engine"></a>
##### Multi-Page Web Crawling Engine
For multi-page web publications, the bootstrapper incorporates an autonomous crawling engine:
- **Kuba Winnicki's *Achtung! Amiga*:** Downloads the root index and all 16 technical subpages (`Copper.html`, `Sprite_Hardware.html`, `Freeing_the_DMA.html`, `More_sprites_in_one_line.html`, `Disappearing_sprites.html`, `UHRES_Display.html`, `Speed_Up_Tricks.html`, `Faster_Chipmem_bus_in_PAL_mode.html`, `Other_Amiga_Native_Hardware.html`, `CD32_Controller.html`, `Battery_Backed_Clock.html`, `Desaturation_Control_Bit.html`, `Video_timings.html`, `Links.html`, `Last_Words.html`, `What_is_this_all_about.html`).
- If the primary live server at `winnicki.net` is unreachable or blocks requests, the crawler automatically switches to the permanent Wayback Machine snapshot mirror.

<a id="processing-raw-documents-into-markdown"></a>
##### Processing Raw Documents into Markdown

When new reference manuals or updated editions are retrieved, use specialized agent skills to convert raw scans and crawled HTML into repository-grade Markdown:

1. **PDF Scans to Markdown ([`pdf-to-markdown`](../.agents/skills/pdf-to-markdown/SKILL.md)):**
   - Leverages Gemini multimodal reasoning to analyze document structure and partition into logical chapters.
   - Extracts and crops circuit diagrams, register maps, and waveforms into high-resolution PNG/SVG assets.
   - Stitches multi-page register tables into GitHub-flavored Markdown tables.
   - Generates Git-tracked multimodal sidecar text files (`<image>.txt`) describing timing diagrams for offline AI inspection.

2. **Web Crawls to Markdown ([`html-to-markdown`](../.agents/skills/html-to-markdown/SKILL.md)):**
   - Crawls multi-page HTML hierarchies (e.g. Kuba Winnicki's *Achtung! Amiga*).
   - Strips legacy table formatting, inline styling, and obsolete navigational chrome.
   - Normalizes cross-chapter hyperlinks into Obsidian internal vault links (`[[Chapter#Section]]`).

<a id="domain-hardware-rag-setup-rag"></a>
#### 3. Domain Hardware RAG Setup (`-Rag`)

Provisions and indexes the local vector database for AI-assisted pair-programming and hardware reference retrieval:

```powershell
.\tools\bootstrap\bootstrap.ps1 -Rag
```

**What It Does:**
- Spawns the local Qdrant container on `http://localhost:6333` (via Docker).
- Uses local FastEmbed (`BAAI/bge-small-en-v1.5`) to embed hardware manuals (`Obsidian/Amiga/Reference/`) and architectural notes (`Obsidian/Amiga/Design/`) into the unified `amiga` collection.
- Maintains `amiga_rag_cache.json` for fast sub-second incremental reindexing.
- For query tools, CLI commands, and FastMCP details, see [Chapter 5: Domain Hardware Knowledge: Local Vector RAG (`amiga-rag`)](#5-domain-hardware-knowledge-local-vector-rag-amiga-rag).

> [!TIP]
> **Fine-Grained CLI Parameters:** For granular control over vector indexing, directory filtering, and database inspection, use the specialized RAG CLI (`.\tools\rag\bin\amiga_rag.ps1` or `python tools/rag/rag_qdrant/cli.py`):
> - `--source <NAME>`: Target partition (`amiga` for hardware reference manuals, `obsidian` for architectural design notes).
> - `--exclude <PATTERNS>`: Exclude directories or file patterns (e.g. `_Private`, `temp`).
> - `--include-dirs <DIRS>`: Restrict indexing to specific subdirectories (e.g. `'01*' '02*'`).
> - `--reindex`: Force full re-indexing of all documents, ignoring the SHA-256 incremental cache.
> - `--list-sources`: Display a summary table of all indexed sources with document and vector counts.
> - `--status`: Inspect Qdrant database connectivity and total collection size.
>
> ```powershell
> # Inspect Qdrant database status and vector counts:
> .\tools\rag\bin\amiga_rag.ps1 --status
>
> # Force full re-indexing of a specific reference manual directory:
> .\tools\rag\bin\amiga_rag.ps1 "Obsidian/Amiga/Reference" --source amiga --reindex
> ```

<a id="code-knowledge-graph-setup-graphify"></a>
#### 4. Code Knowledge Graph Setup (`-Graphify`)

Generates the Abstract Syntax Tree (AST) code knowledge graph connecting active Rust crates and reference implementations:

```powershell
.\tools\bootstrap\bootstrap.ps1 -Graphify
```

**What It Does:**
- Parses Rust source code (`crates/`) and reference C++ code (`ref_src/vAmiga`).
- Maps structs, functions, modules, and cross-codebase dependencies into `graphify-out/`.
- Enables cross-codebase structural queries, call hierarchies, and architectural validation.
- For query tools and incremental update commands, see [Chapter 6: Code Structure & Relationships: Graphify](#6-code-structure-relationships-graphify).

> [!TIP]
> **Fine-Grained CLI Parameters:** For detailed control over AST extraction, graph inspection, and semantic queries, use the specialized `graphify` CLI directly:
> - `graphify update .`: Incrementally re-scan and index only modified files in 1–2 seconds.
> - `graphify extract <PATH> --code-only`: Headless local AST extraction without requiring external LLM API keys.
> - `graphify query "<QUESTION>"`: Query symbol dependencies, call hierarchies, and architectural boundaries.
> - `graphify path "<A>" "<B>"`: Trace the shortest dependency or call path between two types or functions.
> - `graphify explain "<CONCEPT>"`: Extract a focused subgraph explaining a module or subsystem.
> - `graphify export [html|obsidian|wiki]`: Export interactive D3 visualizations, Obsidian canvas notes, or wiki articles.
>
> ```powershell
> # Fast incremental AST update after editing crates:
> graphify update .
>
> # Query call hierarchy and symbol relationships:
> graphify query "How is Agnus connected to the CPU bus?"
> ```

---

<a id="6-domain-hardware-knowledge-local-vector-rag-amiga-rag"></a><a id="6-domain-hardware-knowledge--local-vector-rag-amiga-rag"></a><a id="5-domain-hardware-knowledge-local-vector-rag-amiga-rag"></a><a id="5-domain-hardware-knowledge--local-vector-rag-amiga-rag"></a><a id="a-domain-hardware-knowledge-local-vector-rag-amiga-rag"></a>
## 5. Domain Hardware Knowledge: Local Vector RAG (`amiga-rag`)
- **Knowledge Base Scope:** Connects to local Qdrant database (`http://localhost:6333`, collection: `amiga`), indexing Commodore Hardware Reference Manuals, M68000 PRMs, technical specs, and design specs under `Obsidian/Amiga/`.
- **Vector Database Architecture:**
  - Local Qdrant instance on `http://localhost:6333`.
  - Unified `amiga` collection partitioned by source tags: `amiga` for hardware reference manuals, `obsidian` for architectural design notes.
  - Local FastEmbed (`BAAI/bge-small-en-v1.5`), 100% offline with zero external cloud API keys required.
  - Incremental cache `amiga_rag_cache.json` tracks SHA-256 hashes of individual files, reindexing modified documents in $< 1$ second while skipping unchanged files.
- **Automated Reindexing Trigger ([`amiga-rag.md`](../.agents/rules/amiga-rag.md)):**
  - Reindexing is **fully automated**: whenever hardware documentation, reference guides, or design notes under `Obsidian/Amiga/` are added, modified, or reorganized, incremental reindexing is triggered automatically.
- **Provisioning & Manual Reindexing:**
  ```powershell
  # Set up Qdrant Docker container and index all documentation:
  .\tools\bootstrap\bootstrap.ps1 -Rag

  # Or trigger incremental reindexing directly:
  .\tools\rag\bin\amiga_rag.ps1 "Obsidian/Amiga/Reference" --source amiga
  .\tools\rag\bin\amiga_rag.ps1 "Obsidian/Amiga/Design" --source obsidian
  ```
- **Query Tools & FastMCP:**
  - Query via MCP tool: `rag_search(query="<topic>", sources=["amiga", "obsidian"])`.
  - Fast CLI search across hardware manuals:
    ```powershell
    python tools/harness/rag_search.py "Agnus blitter line mode minterm" --source amiga
    ```
  - Fast CLI search across architectural design specs:
    ```powershell
    python tools/harness/rag_search.py "Color Clock CCK phases memory bus wait states" --source obsidian
    ```
  - Check database status and document count:
    ```powershell
    python tools/rag/rag_qdrant/cli.py status
    ```

---

<a id="7-code-structure-relationships-graphify"></a><a id="7-code-structure--relationships-graphify"></a><a id="6-code-structure-relationships-graphify"></a><a id="6-code-structure--relationships-graphify"></a><a id="b-code-structure-relationships-ast-knowledge-graph-graphify"></a>
## 6. Code Structure & Relationships: Graphify
- **What is Indexed:** Graphify parses Abstract Syntax Trees (AST), symbol relationships, and call hierarchies across two distinct codebases:
  - **Active Emulator Crates (`crates/`):** Core Rust workspace (`m68000`, `memory_bus`, `debugger`, `gui`, `config`, `rtc`, `test_runner`).
  - **Reference Emulator Sources (`ref_src/`):** Clean C++ reference implementation (`ref_src/vAmiga`) and external test harnesses.
- **Incremental Knowledge Graph Updates ([`graphify.md`](../.agents/rules/graphify.md)):**
  - Graphify maintains an AST cache that extracts only modified files in 1–2 seconds without LLM calls.
  - After modifications to code in `crates/` or `ref_src/`, run `graphify update .` from the repository root (or via `.\tools\bootstrap\bootstrap.ps1 -Graphify`).
  - Keeps a single unified knowledge graph in `graphify-out/` connecting active emulator crates and reference implementations.
- **Query Tools:**
  - `graphify query "<question>"`: Query symbol dependencies, call hierarchies, and architectural boundaries.
  - `graphify path "<A>" "<B>"`: Trace the shortest dependency or call path between two types or functions.
  - `graphify explain "<concept>"`: Extract a focused subgraph explaining a subsystem or module.

---

<a id="8-test-suite-verification-framework"></a><a id="8-test-suite--verification-framework"></a><a id="7-test-suite-verification-framework"></a><a id="7-test-suite--verification-framework"></a><a id="6-test-suite-verification-framework"></a>
## 7. Test Suite & Verification Framework

The emulator relies on a multi-tiered verification framework to guarantee 100% cycle-exact fidelity against real Motorola 68000 silicon and Amiga 500 hardware:

<a id="a-m68000-singlesteptests-physical-hardware-silicon-verification"></a>
### A. M68000 SingleStepTests (Physical Hardware Silicon Verification)

The CPU core is validated against **Tom Harte's `SingleStepTests-680x0`** suite (`ref_src/SingleStepTests-680x0/68000/v1/`), consisting of 124 per-instruction test files and ~1,000,000 randomized test vectors captured directly from physical 68000 silicon pins.

#### Execution Modes

- **Fast Smoke Test (Sampled):** By default, each instruction suite evaluates a sampled subset of 50 test cases (~5 seconds total):
  ```powershell
  # Run sampled SingleStepTests across all implemented opcodes:
  cargo test -p test_runner --test test_singlestep

  # Run tests for a specific instruction or family:
  cargo test -p test_runner --test test_singlestep test_nop
  cargo test -p test_runner --test test_singlestep test_add_b
  cargo test -p test_runner --test test_singlestep test_move_w
  ```

- **Full Exhaustive Verification (`SINGLESTEP_FULL`):** Setting `SINGLESTEP_FULL=1` disables sampling limits and executes **100% of all ~1,000,000 test vectors** across all 124 suites in parallel (~5–6 seconds total):
  - **PowerShell (Windows):**
    ```powershell
    $env:SINGLESTEP_FULL = "1"; cargo test -p test_runner --test test_singlestep
    ```
  - **Bash / Linux / macOS:**
    ```bash
    SINGLESTEP_FULL=1 cargo test -p test_runner --test test_singlestep
    ```

- **Custom Sample Limit (`SINGLESTEP_LIMIT`):** To evaluate an arbitrary sample size (e.g. 200 or 500 test cases per opcode):
  ```powershell
  $env:SINGLESTEP_LIMIT = "500"; cargo test -p test_runner --test test_singlestep
  ```

- **Automated Archive Decompression (`tools/bootstrap/bootstrap.ps1 -Sources`):**
  Running `.\tools\bootstrap\bootstrap.ps1 -Sources` automatically scans for compressed archives (`*.json.gz`, `*.gz`, or `.zip`) under `ref_src/SingleStepTests-680x0/` and decompresses them into native `.json` files in `68000/v1/` using native .NET decompression (zero external dependencies).

<a id="b-cartesian-dma-contention-verification"></a>
### B. Cartesian DMA Contention Verification

Validates cycle-exact M68000 micro-stepping and wait-state handling under Agnus DMA bus contention across the full combinatorial Cartesian product:

- **Address Permutations ($2^k$):** Sweeps all role assignments of memory cells touched by the instruction (`ChipRam` vs `FastRam`).
- **DMA Schedule Permutations ($2^M$):** Sweeps every bit pattern of stalled vs free CCK slots across the execution window.
- **Asserted State Constraints:**
  1. *Cycle Invariance:* $C = C_0 + 2 \times \text{wait\_states}$
  2. *Fast RAM Immunity:* $C = C_0$ with zero wait states when memory addresses point to Fast RAM.
  3. *State Invariance:* Register values and RAM contents are 100% bit-identical to the uncontended golden run.

```powershell
# Run full Cartesian DMA contention test suite:
cargo test -p test_runner --test test_dma_cartesian

# Run specific sub-suite:
cargo test -p test_runner --test test_dma_cartesian test_dma_cartesian_system_and_traps
```

<a id="c-automated-architecture-rules-compliance"></a>
### C. Automated Architecture Rules Compliance

Enforces architectural rules defined in [`AGENTS.md`](../AGENTS.md) (code formatting, file size limits $\le 800$ lines, zero runtime panics, zero custom macros, path privacy, and inlining rules):
```powershell
cargo test -p test_runner --test test_architecture_rules
```

<a id="d-cli-diagnostic-tools-regression-tracking"></a><a id="d-cli-diagnostic-tools--regression-tracking"></a>
### D. CLI Diagnostic Tools & Regression Tracking

The `test_runner` crate provides a standalone CLI tool for inspecting coverage matrices, live failure diagnostics, and detecting regressions:

> [!TIP]
> Running `cargo run -p test_runner` without arguments (or with `--help`) prints the complete usage manual and available command options.

```powershell
# Display global pass/fail matrix and coverage summary across all opcodes:
cargo run -p test_runner -- --summary

# Detect regressions and fixed tests compared to previous run:
cargo run -p test_runner -- --diff

# Execute a single opcode suite directly with live diagnostic failure output:
cargo run -p test_runner -- --suite ADD.b
```

- 🔴 **Regressions:** Tests that previously passed but now fail are highlighted with `⚠️ [REGRESSION DETECTED]`.
- 🟢 **Improvements:** Tests that previously failed but now pass are highlighted with `🎉 [PROGRESS / FIX]`.

<a id="e-vamigats-verification-framework-golden-viewport-testing"></a><a id="e-vamigats-verification-framework--golden-viewport-testing"></a><a id="e-vamigats-subsystem-whole-machine-verification-preview"></a><a id="e-vamigats-subsystem--whole-machine-verification-preview"></a>
### E. vAmigaTS Verification Framework & Golden Viewport Testing

The emulator features a dedicated verification harness and execution runner for the **vAmiga Test Suite (vAmigaTS)** (`ref_src/vAmigaTS/`, 2,077 test directories), providing cycle-exact chipset verification against golden 716×285 24-bit RGB `.raw` viewport captures recorded from real Amiga 500 hardware.

#### 1. How vAmigaTS Verification Works
Each test under `ref_src/vAmigaTS/<Category>/<Subcategory>/<TestName>/` encapsulates a complete hardware test scenario:
- **Test Binary / ADF:** A bootable floppy disk image (`.adf`) or raw M68000 binary payload injected into Chip RAM at entry point `$1000`.
- **Test Script (`test.retrosh`):** Declarative shell script specifying memory configuration, initial register values, frames to simulate, and optional viewport cutouts.
- **Golden Reference Viewport (`screenshot.raw`):** Exact 24-bit RGB uncompressed pixel capture (716 × 285 pixels = 612,180 bytes) produced by clean silicon.
- **Verification Pipeline:**
  1. Sets up minimal OS stub interrupt and library vectors (`ExecBase` at `$0004`, `GfxBase`).
  2. Steps the machine loop for the scripted number of PAL frames (default: 8–16 frames).
  3. Extracts Denise's 24-bit RGB frame buffer and compares it pixel-by-pixel against `screenshot.raw`.
  4. Reports matching pixel count, mismatch coordinates, and actual vs expected RGB values.

#### 2. Provisioning Prerequisite
Ensure reference assets are extracted prior to running vAmigaTS suites:
```powershell
.\tools\bootstrap\bootstrap.ps1 -Sources
```

#### 3. Subsystem Integration Test Suites (Cargo)
Dedicated regression tests in `crates/test_runner/tests/` exercise targeted custom chip components:
```powershell
# Run general vAmiga test infrastructure & catalog discovery tests:
cargo test -p test_runner --test test_vamiga_runner

# Run low-level test harness, stub vectors, and raw buffer matcher tests:
cargo test -p test_runner --test test_vamiga_harness

# Verify Copper coprocessor timing, hazards, and halt clusters:
cargo test -p test_runner --test test_vamiga_copper

# Verify Blitter DMA channels, line drawing mode, and minterms:
cargo test -p test_runner --test test_vamiga_blitter

# Verify Denise display window (DIW) clipping and bitplane color decoding:
cargo test -p test_runner --test test_vamiga_denise

# Verify Paula audio channels, period division, and interrupt timings:
cargo test -p test_runner --test test_vamiga_paula
```

#### 4. Interactive CLI Runner (`test_runner vamiga`)
Execute tests with live pixel diff reporting, category filtering, and frame control:
```powershell
# Execute a specific test by name with live pixel mismatch reporting:
cargo run -p test_runner -- vamiga --test coptim1

# Execute a specific test with verbose pixel difference coordinates:
cargo run -p test_runner -- vamiga --test coptim1 -v

# Run all runnable tests in a specific custom chip category:
cargo run -p test_runner -- vamiga --category Copper
cargo run -p test_runner -- vamiga --category Blitter
cargo run -p test_runner -- vamiga --category Denise
cargo run -p test_runner -- vamiga --category Paula

# Override the number of frames simulated (e.g. 16 frames):
cargo run -p test_runner -- vamiga --test coptim1 --frames 16

# Limit batch execution count during active development:
cargo run -p test_runner -- vamiga --category Copper --max-tests 5

# Inspect runnable vs deferred test suite breakdown across roadmap phases:
cargo run -p test_runner -- vamiga --list-deferred
```

#### 5. Diagnostic Diff Reporting & Root-Cause Guidelines
When a test fails, the runner outputs a structured mismatch summary:
```text
❌ FAIL: 5500/204060 mismatched pixels (2.69%)
    Mismatch at (128, 42): actual=(0, 0, 0), expected=(255, 128, 0)
    Mismatch at (129, 42): actual=(0, 0, 0), expected=(255, 128, 0)
```

> [!IMPORTANT]
> **Zero Coordinate Nudging:** Per [`.agents/rules/structural-root-cause.md`](../.agents/rules/structural-root-cause.md), never fix pixel mismatches by nudging display coordinates ($\pm 1$ offset hacks). Investigate the underlying silicon timing: Color Clock phase alignment (`CCK1` vs `CCK2`), Copper instruction strobe delay, DMA slot contention, or display window (`DIWSTRT`/`DIWSTOP`) clipping.


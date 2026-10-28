# Development Environment and Reference Setup

This operational reference contains setup commands and provisioning details. Start with the [developer overview](../../docs/developers.md) for the project map and the [testing overview](../../docs/testing.md) for verification scope. Run commands from the repository root; current program help and owning tool READMEs provide their detailed interfaces.

> [!IMPORTANT]
> **Foundational Architecture:** Before modifying or extending emulator subsystems, review the [**Core Architecture & Hardware Execution Model**](../../docs/architecture.md). It formalizes the cycle-exact Color Clock phases (`CCK1`/`CCK2`), shared bus contention, Gary arbitration, Big-Endian invariance, and decoupled subsystem ownership. Comprehensive silicon specifications, register allocations, and circuit schematics reside in [`Obsidian/Amiga/Design/`](../../Obsidian/Amiga/Design).

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

The emulator uses external test suites, hardware measurements, and reference implementations. These sources serve different purposes; a hardware capture can establish observed signals, while emulator and FPGA code provide models to compare against them:

- **[Tom Harte SingleStepTests](https://github.com/SingleStepTests/680x0):** Software-generated M68000 instruction reference vectors.
- **[vAmigaTS](https://github.com/dirkwhoffmann/vAmigaTS):** Amiga chipset regression tests and reference captures.
- **[vAmiga](https://github.com/dirkwhoffmann/vAmiga):** C++ reference emulator for chipset behavior and timing.
- **[Amiga Test Kit](https://github.com/keirf/amiga-stuff):** Bootable hardware diagnostics.
- **[Amiga Replacement Project](https://github.com/nonarkitten/amiga_replacement_project):** Work-in-progress Verilog chip reconstructions; its Paula core still calls for verification on real hardware.
- **[MiniMig](https://github.com/cristianoag/minimig):** FPGA Amiga 500 implementation for comparing chipset coordination.
- **[MiniMig-AGA for MiSTer](https://github.com/MiSTer-devel/Minimig-AGA_MiSTer):** Later FPGA core supporting OCS, ECS, and AGA configurations.
- **[Techtravels.org](https://www.techtravels.org/):** Amiga hardware investigations, including logic-analyzer measurements and Paula circuit analysis; distinguish measured pins from inferred internal behavior.

> [!NOTE]
> **Graphify Code Knowledge Graph Integration:** The single repository graph (`graphify-out/`) covers project code plus the bootstrapped `ref_src/vAmiga/`, `ref_src/minimig/`, and `ref_src/amiga_replacement_project/`. The bootstrap disables Git ignore rules for extraction, while `.graphifyignore` limits the reference set and excludes generated data. Use graph results to find source locations, then verify the original code; FPGA code is a model, not a hardware measurement. See [Chapter 6](#6-code-structure-relationships-graphify).

Because these test assets comprise multi-gigabyte datasets (~6.5 GB uncompressed), they reside outside Git history in `ref_src/` and `tools/`.

<a id="pinned-upstream-sources-summary"></a>
### Pinned Upstream Sources Summary

| Repository / Asset | Local Path | Upstream Repository & URL | Pinned Version / Release |
| :--- | :--- | :--- | :--- |
| **Tom Harte SingleStepTests** | `ref_src/SingleStepTests-680x0/` | [SingleStepTests/680x0](https://github.com/SingleStepTests/680x0) | **Format v1** (`68000/v1/`), 124 opcode suites |
| **vAmiga C++ Emulator** | `ref_src/vAmiga/` | [dirkwhoffmann/vAmiga](https://github.com/dirkwhoffmann/vAmiga) | **Release v4.5** |
| **vAmiga Test Suite (vAmigaTS)** | `ref_src/vAmigaTS/` | [dirkwhoffmann/vAmigaTS](https://github.com/dirkwhoffmann/vAmigaTS) | **`master`** (2,077 test directories) |
| **Amiga Test Kit** | `tools/AmigaTestKit/AmigaTestKit.adf` | [keirf/amiga-stuff](https://github.com/keirf/amiga-stuff) | **Release v1.21** (`AmigaTestKit.adf`) |
| **MiniMig** | `ref_src/minimig/` | [cristianoag/minimig](https://github.com/cristianoag/minimig) | **`89c2643101860be4caa6ffb275e9bdd7c60d61d2`** |
| **Amiga Replacement Project** | `ref_src/amiga_replacement_project/` | [nonarkitten/amiga_replacement_project](https://github.com/nonarkitten/amiga_replacement_project) | **`143ad5f1b5acae093ea66186e832a8ab4fb1bc0f`** |

The two FPGA archives are optional individual selections in `bootstrap_sources.ps1`; the coordinator's `-Sources` switch includes both. Their local `_bootstrap_source.json` files record the archive hash, upstream revision, and per-file hashes. They are Git-ignored research references. The [hardware researcher](../../.codex/agents/hardware_researcher.toml) also consults measured [Techtravels Paula/DMAL evidence](https://www.techtravels.org/2026/08/from-flux-to-filenames-335ms-inside-paula/) through web search when needed; web pages are not copied into the documentation RAG index.

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
   - **Scope:** Cycle-exact algorithmic analysis of 68000 non-restoring integer division. Formulates precise cycle calculations compared with software-generated SingleStep reference vectors.

7. **Undocumented Features of OCS, ECS and AGA Chipsets (Kuba Winnicki / Achtung! Amiga, 2002)**
   - **Location:** `Obsidian/Amiga/Reference/Undocumented features of OCS, ECS and AGA chipsets.md`
   - **Scope:** 16-chapter investigation into silicon quirks: Copper hazards, sprite demultiplexing, DMA slot arbitration, UHRES display modes, and video beam timing anomalies.

> [!TIP]
> Information and automated scripts for downloading, bootstrapping, and converting these external reference materials into Markdown are provided in **[Chapter 4: Archival Reference Documentation Provisioning (`-Documentation`)](#archival-documentation-provisioning-documentation)** (`.\tools\bootstrap\bootstrap.ps1 -Documentation`).

---

<a id="4-bootstrapping-overview-toolsbootstrapbootstrapps1"></a><a id="4-bootstrapping-overview--toolsbootstrapbootstrapps1"></a><a id="5-bootstrapping-overview-toolsbootstrapbootstrapps1"></a><a id="5-bootstrapping-overview--toolsbootstrapbootstrapps1"></a><a id="4-external-reference-downloads-provisioning"></a><a id="4-external-reference-downloads--provisioning"></a><a id="4-archival-documentation-reference-downloads"></a><a id="4-archival-documentation--reference-downloads"></a><a id="2-optional-bootstrapping-overview"></a><a id="bootstrapping-overview"></a>
## 4. Bootstrapping & External Reference Provisioning (`tools/bootstrap/bootstrap.ps1`)

Bootstrapping is **strictly optional**. A freshly cloned repository compiles and runs the standalone desktop GUI or WebAssembly canvas out of the box using only the stable Rust toolchain.

However, specialized development and verification workflows—such as running exhaustive 124-suite M68000 SingleStep tests, validating against golden vAmigaTS visual viewports, querying the offline Qdrant RAG vector database, or exploring cross-codebase call graphs—require external reference assets, technical literature, or tooling services.

The centralized bootstrapper (`tools/bootstrap/bootstrap.ps1`) orchestrates all provisioning workflows.

<a id="full-development-setup-all"></a><a id="bootstrapper-commands"></a>
### Complete Development Setup (`-All`)

To provision the repository-managed reference assets and tooling indexes in a single run, use the `-All` switch. Its RAG phase still starts this repository's independent Qdrant container. When using the separate qdrant-rag repository, run the other provisioning switches individually and follow the RAG setup below:

```powershell
.\tools\bootstrap\bootstrap.ps1 -All
```

| Mode | Switch | When Needed | What It Provisions |
| :--- | :--- | :--- | :--- |
| **Full Setup** | `-All` | Provision repository-managed development assets | Executes five tiers: **Sources** $\to$ **Graphify** $\to$ **Documentation** $\to$ **RAG** $\to$ **Assembler**. It uses the independent Qdrant container, not the external repository's Compose setup. |
| **Verification Testbeds** | `-Sources` | Provision local test and research references | Downloads external test suites, software-generated M68000 vectors (`SingleStepTests-680x0`), vAmiga/vAmigaTS, Amiga Test Kit, MiniMig, and Amiga Replacement Project |
| **External Reference Scans** | `-Documentation` | External reference scans and manual archives | Downloads original archival PDF scans and OEM technical reference manuals (HRM, TRM, PRM, UM, and technical articles) |
| **Documentation & RAG** | `-Rag` | Legacy repository-managed Qdrant bootstrap | Starts its own Qdrant container and indexes the Amiga documentation scopes through the external `rag_qdrant` CLI; it does not install or configure the separate RAG service. |
| **Code Knowledge Graph** | `-Graphify` | Codebase structural navigation, call hierarchy, and symbol dependency analysis | Builds or updates one graph in `graphify-out/` with project code and the three selected reference implementations |

---

### Granular Provisioning Workflows

Each provisioning phase can also be executed independently on demand:

<a id="automated-provisioning-toolsbootstrapbootstrap_sourcesps1"></a><a id="sources-provisioning-sources"></a><a id="reference-sources-lifecycle-steps"></a><a id="automation-lifecycle-steps"></a>
#### 1. Verification Testbeds Provisioning (`-Sources`)

Automates downloading, unpacking, decompressing, and validating external test assets and reference code:

```powershell
.\tools\bootstrap\bootstrap.ps1 -Sources
# Or select only the two FPGA repositories:
.\tools\bootstrap\bootstrap_sources.ps1 -MiniMig -AmigaReplacementProject
```

##### Automation Lifecycle Steps

When external test sources are provisioned via `.\tools\bootstrap\bootstrap.ps1 -Sources`, the automated bootstrapper executes the following lifecycle steps:

1. **Zip Archive Expansion:** Scans `ref_src/SingleStepTests-680x0/` for `.zip` archives and unpacks them into place.
2. **Gzip Decompression:** Scans for `.json.gz` or `.gz` compressed test archives and decompresses them into native `.json` files using .NET `GZipStream` (zero external dependencies).
3. **Directory Canonicalization:** Migrates any loose `.json` test suites from `68000/` into the canonical `68000/v1/` directory.
4. **Presence & Completeness Validation:** Verifies the 124 SingleStep suites, Amiga Test Kit, vAmiga, and vAmigaTS. For the FPGA references, it checks the pinned archive SHA-256 before extraction and verifies the per-file manifest on repeat runs.

> [!TIP]
> **Fine-Grained Execution:** When you need direct execution, testing, or pipeline debugging without invoking the coordinator wrapper, invoke the dedicated sources bootstrapper directly:
> ```powershell
> .\tools\bootstrap\bootstrap_sources.ps1 -List
> ```

<a id="bootstrapping-raw-archival-sources-toolsbootstrapbootstrap_documentationps1"></a><a id="archival-documentation-provisioning-documentation"></a><a id="upstream-archival-reference-documents"></a><a id="multi-source-fallback-matrix-error-resilience"></a><a id="multi-source-fallback-matrix--error-resilience"></a>
#### 2. Archival Reference Documentation Provisioning (`-Documentation`)

Provisions original PDF scans, OEM technical manuals, and archival web articles for developers inspecting raw schematics or re-running OCR pipelines:

Each book's downloads go into `Obsidian/Amiga/Reference/<Document_Name>-tmp/`
(or `<Destination>/<Document_Name>-tmp/` with a custom destination). Conversion
keeps its `workspace/` there. `-Markdown` leaves final results in that workspace:
`14_link_toc/` for PDF and `html_to_markdown/` for HTML. Add `-Publish` to copy
finished Markdown and assets to the sibling `<Document_Name>/` directory without
the suffix. The destination must be absent or empty; existing files, hidden entries
and links cause an error before downloads or conversion. The converter checks again
before publishing and never replaces an occupied directory. Existing downloads in
unsuffixed directories are not moved or
reused automatically; the `-tmp` directories remain available for inspection and
resuming conversion. Neither pipeline accepts `--output-dir` or automatically
mirrors HTML into `Reference`. Direct CLI publication uses the boolean `--publish`
and derives the book destination from its enclosing `<Document_Name>-tmp/` directory.

```powershell
.\tools\bootstrap\bootstrap.ps1 -Documentation
# Convert locally, then optionally publish to an empty book directory:
.\tools\bootstrap\bootstrap.ps1 -Documentation -Markdown -Publish
```

<a id="multi-page-web-crawling-engine"></a>
##### Multi-Page Web Crawling Engine
For multi-page web publications, the bootstrapper incorporates an autonomous crawling engine:
- **Kuba Winnicki's *Achtung! Amiga*:** Downloads the root index and all 16 technical subpages (`Copper.html`, `Sprite_Hardware.html`, `Freeing_the_DMA.html`, `More_sprites_in_one_line.html`, `Disappearing_sprites.html`, `UHRES_Display.html`, `Speed_Up_Tricks.html`, `Faster_Chipmem_bus_in_PAL_mode.html`, `Other_Amiga_Native_Hardware.html`, `CD32_Controller.html`, `Battery_Backed_Clock.html`, `Desaturation_Control_Bit.html`, `Video_timings.html`, `Links.html`, `Last_Words.html`, `What_is_this_all_about.html`).
- If the primary live server at `winnicki.net` is unreachable or blocks requests, the crawler automatically switches to the permanent Wayback Machine snapshot mirror.

<a id="processing-raw-documents-into-markdown"></a>
##### Processing Raw Documents into Markdown

The bootstrap converters prepare the initial reference knowledge base from raw scans and downloaded HTML. PDF preparation, HTML conversion and book review have separate entry points in the [bootstrap instructions](../../tools/bootstrap/AGENTS.md). Shared Codex authentication, requests and caching are described in the [conversion client README](../../tools/bootstrap/conversion/README.md). Automatic tests follow the [bootstrap converter scope](../rules/unit-testing-policy.md#bootstrap-converter-scope). Run only the fragment requested by the user; the user assesses generated content.

1. **PDF preparation ([`pdf-to-markdown`](../../tools/bootstrap/pdf-to-markdown/README.md)):** Stage 01 prepares a native/OCR text layer, Stage 02 extracts page images and positioned text, and Stages 03–07 prepare ordered objects, tables, crops and callouts. See the program README for current stage commands and outputs.
2. **Book review ([`book-to-markdown`](../../tools/bootstrap/book-to-markdown/README.md)):** Review prepared pages, roles and source selections. A standalone recipe-based Markdown reconstruction bootstrap is planned; local editor acceptance does not establish a portable recipe.
3. **HTML conversion ([`html-to-markdown`](../../tools/bootstrap/html-to-markdown/README.md)):** Convert downloaded HTML files or crawls into Markdown and referenced assets. Its prompt owns transcription instructions.

The dispatcher is [`bootstrap_documentation.ps1`](../../tools/bootstrap/bootstrap_documentation.ps1). Authenticate with `codex login` before model-backed conversion; install dependencies from each program's requirements file.

<a id="domain-hardware-rag-setup-rag"></a>
#### 3. Domain Hardware RAG Setup (`-Rag`)

The Qdrant server configuration and `rag_qdrant` CLI live in the separate [qdrant-rag repository](https://github.com/tomek-anuszkiewicz/qdrant-rag). Clone and configure it before using RAG from this project:

```powershell
$RagRepo = "<path-to-qdrant-rag>"
git clone https://github.com/tomek-anuszkiewicz/qdrant-rag.git $RagRepo
Set-Location $RagRepo
python -m pip install -r .\rag-qdrant\requirements.txt
docker compose up -d
```

Start Docker Desktop before running `docker compose up -d`. Follow the cloned repository's README for any additional configuration required by the version you checked out. Qdrant stores its vectors in that repository's persistent Docker volume or bind mount; the CLI also needs a JSON file for incremental index state.

Add the cloned repository's `rag-qdrant\bin` directory to `PATH` so `rag_qdrant` is available to the Amiga scripts and MCP adapter. For the current PowerShell session on Windows:

```powershell
$env:PATH = "$RagRepo\rag-qdrant\bin;$env:PATH"
Get-Command rag_qdrant.bat
$env:RAG_INDEX_JSON = "<shared-rag-index-state-file>"
rag_qdrant.bat --status --json
```

Set `RAG_INDEX_JSON` to the service's canonical writable state file for indexing. Search, status and source listing use the service configuration without a state-file argument. Add the launcher directory to your persistent user `PATH` if other shells or IDE sessions need it. Run the three Amiga indexing commands in [Chapter 5](#5-domain-hardware-knowledge-local-vector-rag-amiga-rag) from the Amiga repository root after the documentation audit succeeds. The `-Rag` and `-All` bootstrap switches still use this repository's older, independent Qdrant container helper and may conflict with the external repository's Compose container on ports 6333/6334. Use the external repository's Compose setup and the direct CLI commands for the shared database.

<a id="code-knowledge-graph-setup-graphify"></a>
#### 4. Code Knowledge Graph Setup (`-Graphify`)

Generates one Abstract Syntax Tree (AST) code graph connecting project sources and the three selected reference implementations:

```powershell
.\tools\bootstrap\bootstrap.ps1 -Graphify
```

**What It Does:**
- Parses code under `crates/`, `tools/`, `tests/`, `benchmarks/`, and `examples/`, together with vAmiga 4.5, MiniMig, and Amiga Replacement Project under `ref_src/`.
- Excludes SingleStep, vAmigaTS, other reference trees, generated book data, and JSON test vectors through `.graphifyignore`. The first extraction uses `--no-gitignore` so Git-ignored reference files are included; subsequent updates reuse this setting.
- Maps code symbols and relationships into `graphify-out/graph.json`; the separate documentation RAG remains the route for hardware manuals.
- Enables cross-codebase structural queries, call hierarchies, and architectural validation.
- For query tools and incremental update commands, see [Chapter 6: Code Structure & Relationships: Graphify](#6-code-structure-relationships-graphify).

---

<a id="6-domain-hardware-knowledge-local-vector-rag-amiga-rag"></a><a id="6-domain-hardware-knowledge--local-vector-rag-amiga-rag"></a><a id="5-domain-hardware-knowledge-local-vector-rag-amiga-rag"></a><a id="5-domain-hardware-knowledge--local-vector-rag-amiga-rag"></a><a id="a-domain-hardware-knowledge-local-vector-rag-amiga-rag"></a>
## 5. Domain Hardware Knowledge: Local Vector RAG (`amiga-rag`)
- **Knowledge Base Scope:** Connects to local Qdrant database (`http://localhost:6333`, collection: `projects_docs`), indexing Commodore Hardware Reference Manuals, M68000 PRMs, technical specs, and design specs under `Obsidian/Amiga/`.
- **Vector Database Architecture:**
  - Local Qdrant instance on `http://localhost:6333`.
  - Unified `projects_docs` collection partitioned by source tags: `amiga` for this repository's manuals and design specifications, `devnotes` for separately maintained developer notes.
  - Local FastEmbed (`BAAI/bge-small-en-v1.5`), 100% offline with zero external cloud API keys required.
  - The CLI state file named by `RAG_INDEX_JSON` tracks SHA-256 hashes of individual files, reindexing modified documents while skipping unchanged files.
  - **Incremental Reindexing Trigger ([`amiga-rag.md`](../rules/amiga-rag.md)):**
    - At the end of a documentation batch, run the canonical CLI commands. SHA-256 caching skips unchanged files; there is no file-save watcher.
- **Provisioning & Manual Reindexing:**
  ```powershell
  # Set the shared CLI state file, then index each project Markdown scope.
  $env:RAG_INDEX_JSON = "<shared-rag-index-state-file>"
  rag_qdrant docs --source amiga --index-json $env:RAG_INDEX_JSON
  rag_qdrant "Obsidian/Amiga/Design" --source amiga --index-json $env:RAG_INDEX_JSON
  rag_qdrant "Obsidian/Amiga/Reference" --source amiga --index-json $env:RAG_INDEX_JSON
  ```
- **Query Tools & FastMCP:**
   - Query via MCP tool: `rag_search(query="<topic>", sources=["amiga", "devnotes"])`. The Amiga project only retrieves `devnotes`; its owning project indexes it.
  - Fast CLI search across hardware manuals:
    ```powershell
    rag_qdrant search "Agnus blitter line mode minterm" --source amiga --limit 5 --json
    ```
  - Fast CLI search across architectural design specs:
    ```powershell
    rag_qdrant search "Color Clock CCK phases memory bus wait states" --source devnotes --limit 5 --json
    ```
  - Check database status and document/source counts:
    ```powershell
    rag_qdrant --status --json
    rag_qdrant --list-sources --json
    ```

---

<a id="7-code-structure-relationships-graphify"></a><a id="7-code-structure--relationships-graphify"></a><a id="6-code-structure-relationships-graphify"></a><a id="6-code-structure--relationships-graphify"></a><a id="b-code-structure-relationships-ast-knowledge-graph-graphify"></a>
## 6. Code Structure & Relationships: Graphify
- **What is Indexed:** One graph at `graphify-out/graph.json` holds code from `crates/`, `tools/`, `tests/`, `benchmarks/`, and `examples/`, plus `ref_src/vAmiga/` (release 4.5), `ref_src/minimig/`, and `ref_src/amiga_replacement_project/`. SingleStep and vAmigaTS are excluded. Graphify recognizes Rust, Python, PowerShell, C++, and Verilog source files, but JSON test vectors are excluded and some source files may yield partial symbols after parser errors.
- **Updates ([Graphify skill](../skills/graphify/SKILL.md)):** Run `.\tools\bootstrap\bootstrap.ps1 -Graphify` to build or update the one graph. The first run extracts with `--no-gitignore --code-only`; later runs use `graphify update .` with the saved setting. No LLM calls are needed for code extraction.
- **Query Tools:**
  - `graphify query "<question>"`: Query the full graph; it does not strictly restrict results to a source directory.
  - `python tools/graphify_query.py "Paula" --scope minimig`: Query only MiniMig nodes and edges. Repeat `--scope` to compare `project`, `vamiga`, `minimig`, and `arp`; filtering occurs before Graphify traverses the graph. The wrapper creates only a disposable filtered view under `.agent/tmp/`, leaving the one stored graph intact.
  - `graphify path "<A>" "<B>"`: Trace the shortest dependency or call path between two types or functions.
  - `graphify explain "<concept>"`: Extract a focused subgraph explaining a subsystem or module.

---

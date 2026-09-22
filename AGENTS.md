# Amiga 500 Emulator Architectural & Engineering Guidelines

This repository contains the cycle-exact Amiga 500 emulator written in Rust.
All agentic pair-programming and automated modifications must adhere strictly to the architectural constraints, execution model, and coding guidelines detailed below.

---

## 1. Operating & Behavioral Rules (`.agents/rules/`)

Operational rules are modularized under `.agents/rules/` with single-responsibility scoping across two tiers:

### A. Universal Invariants (`trigger: always_on`)
- **Audio Voice Transcription** ([`audio-transcription.md`](.agents/rules/audio-transcription.md)): Mandatory spoken language transcript echo before responses.
- **Dynamic Model Advisory** ([`model-reasoning-advisory.md`](.agents/rules/model-reasoning-advisory.md)): Proactive advice on switching between `Medium` and `High`/`Pro` reasoning.
- **Strict Path Privacy** ([`no-external-paths.md`](.agents/rules/no-external-paths.md)): Zero external host paths; use generic placeholders.
- **Specification Compliance** ([`spec-compliance.md`](.agents/rules/spec-compliance.md)): Zero silent divergence; mandatory user conflict escalation before code changes.
- **Hardware Efficiency & Readability** ([`performance-and-readability.md`](.agents/rules/performance-and-readability.md)): Flat execution, zero macros, zero const-generics, contiguous execution, and zero runtime heap allocations in hot paths.
- **Amiga RAG Knowledge Base** ([`amiga-rag.md`](.agents/rules/amiga-rag.md)): Pre-task conceptual retrieval through the Amiga RAG MCP server.
- **Information Hierarchy & Limits** ([`information-hierarchy.md`](.agents/rules/information-hierarchy.md)): Inverted pyramid model, constitutional AGENTS.md ceiling ($\le 14,000$ bytes), and non-redundancy.
- **Unit Testing Policy** ([`unit-testing-policy.md`](.agents/rules/unit-testing-policy.md)): Mandatory unit test coverage for functional/utility logic and public APIs; dedicated `tests/` directories with zero inline tests in `src/`.
- **Immediate Atomic Commits** ([`git-commits.md`](.agents/rules/git-commits.md)): Mandatory atomic commit after every completed task or refactoring; zero uncommitted changes left across turns; Conventional Commits and pre-commit test gates.
- **Structural Root-Cause Resolution** ([`structural-root-cause.md`](.agents/rules/structural-root-cause.md)): Mandatory structural upstream fixes; strict prohibition of local symptom patches (pixel/timing nudges, ad-hoc regexes, special-case branches).

### B. Domain-Specific Rules (`trigger: model_decision`)
- **Language Policy** ([`language-policy.md`](.agents/rules/language-policy.md)): Strict English for all agent responses, plans, artifacts, source code, and commit messages.
- **Design Docs Maintenance** ([`docs-maintenance.md`](.agents/rules/docs-maintenance.md)): Synchronizing `Obsidian/Amiga/Design/` specs with code and pruning draft proposals.
- **Engineering Diary** ([`diary-maintenance.md`](.agents/rules/diary-maintenance.md)): Mandatory chronological narrative logging in `DIARY.md` (Section 10).
- **Roadmap Maintenance** ([`roadmap-maintenance.md`](.agents/rules/roadmap-maintenance.md)): Substrate-first ordering, zero completed items retention, and scorecard sync in `ROADMAP.md`.
- **Vault Linking & Graph Integrity** ([`vault-linking-and-graph-integrity.md`](.agents/rules/vault-linking-and-graph-integrity.md)): Line 1 YAML properties evaluation, dual-layer linking, and zero broken links across `Obsidian/Amiga/Design/`.
- **Source File Size & Cohesion** ([`file-size-and-cohesion.md`](.agents/rules/file-size-and-cohesion.md)): File size $\le 800$ lines in `crates/*/src/` (with recognized exceptions), single responsibility, and flat instructions hierarchy.
- **Opcode Naming & Micro-Steps** ([`opcode-naming.md`](.agents/rules/opcode-naming.md)): Canonical `IDLE` micro-steps, 1:1 opcode files, and dual staging registers (`addr1`/`addr2`).
- **Method Inlining Strategy** ([`method-inlining.md`](.agents/rules/method-inlining.md)): Targeted `#[inline]`, `#[inline(always)]`, and `#[inline(never)]` annotations.
- **Workspace Architecture & Re-Exports** ([`workspace-structure-and-reexports.md`](.agents/rules/workspace-structure-and-reexports.md)): Strictly flat crate layout, named crate roots (`<crate>.rs`), and 3-tier re-export hierarchy.
- **Rust Best Practices** ([`rust-best-practices.md`](.agents/rules/rust-best-practices.md)): Safe borrowing, zero unwraps in runtime, wrapping math, and unit tests.
- **Repro-First Defect Resolution** ([`repro-first.md`](.agents/rules/repro-first.md)): Mandatory isolated failing test before modifying production code.
- **egui & Frontend Best Practices** ([`egui-best-practices.md`](.agents/rules/egui-best-practices.md)): Synchronous state pull, 1:1 layout mapping, bounded time-slicing, WASM/DPI adaptation, and universal in-app documentation.
- **Git Merge Commits & Worktrees** ([`git-merge-commits.md`](.agents/rules/git-merge-commits.md)): Mandatory merge commits on conflict resolution; worktree lifecycle and cleanup.
- **Graphify Knowledge Graph** ([`graphify.md`](.agents/rules/graphify.md)): Mandatory AST query before file inspection; incremental updates via `graphify update .`.
- **Asset Descriptions & Sidecars** ([`asset-descriptions.md`](.agents/rules/asset-descriptions.md)): Git-tracked `<image_path>.txt` technical sidecars for circuit and timing schematics.
- **Parallel Execution & Async Tasks** ([`parallel-execution.md`](.agents/rules/parallel-execution.md)): Non-blocking background tasks, targeted sub-suite testing, and multi-agent workflows.
- **Practitioner Voice & Tone** ([`practitioner-voice-and-tone.md`](.agents/rules/practitioner-voice-and-tone.md)): Hands-on lead architect persona, tech blog standard, and zero academic jargon.

---

## 2. Core Architectural Principles

1. **Target Platforms & Portability**:
   - Compiles seamlessly for native desktop (x86_64, aarch64) and WebAssembly (`wasm32-unknown-unknown`).
   - System-agnostic core: zero direct OS or platform dependencies. Host I/O (display, audio, disks, ROM injection) is handled via external buffers and decoupled interfaces.

2. **Clock & Execution Model (Color Clock Phases)**:
   - Primary synchronization unit: Amiga Color Clock (**CCK**, ~3.54 MHz PAL / ~3.58 MHz NTSC).
   - CPU bus cycles and execution stages are modeled using Color Clock phases: **CCK1** and **CCK2**.
     - $1\ \text{M68000 bus cycle} = 4\ \text{CPU clocks} = 2\ \text{CCK cycles}\ (\text{CCK1} + \text{CCK2})$.
   - Memory access methods (`read_byte`, `read_word`, `write_byte`, `write_word`) return `BusResult::WaitState` when Chip RAM is blocked by custom chip DMA. If blocked, CPU waits additional CCK cycles without advancing its active micro-step.

3. **Decoupled Architecture & Ownership**:
   - Top-level machine struct (`A500`) owns all major subsystems: `Cpu` (`M68000`), `MemoryBus`, `Agnus`, `Denise`, `Paula`, `CiaA`, and `CiaB`, tracking master Color Clocks via monotonic `cck: u64`.
   - **No circular references**: Subsystems must not hold direct pointers or circular handles (`Rc<RefCell<...>>`).
   - All chip coordination, interrupt priority line (IPL 1-6) arbitration, and bus locks are driven in the main machine loop and `MemoryBus`.

4. **Hardware Circuit Simulation & Save States**:
   - Physical circuit simulation: register modifications and bus signals take effect on subsequent clock phases or cycles rather than propagating instantaneously.
   - Subsystem state structs (`CpuState`, custom chip states) are decoupled from runtime handles, fully queryable (read-only snapshots), and implement `serde::Serialize` and `serde::Deserialize`.

5. **Substrate-First Invariant (Physical Causal Ordering)**:
   - Subsystem implementation, roadmaps, and verification plans strictly follow physical electronic causality: Layer 0 (Bus Arbitration, Clock Phases, Contention) $\to$ Layer 1 (Autonomous DMA: Copper, Blitter) $\to$ Layer 2 (Display Pipeline: Denise) $\to$ Layer 3 (Peripherals: Paula, CIAs) $\to$ Layer 4 (Firmware & Exec). Zero folder-tree taxonomic planning.

---

## 3. Rust Systems & Machine Invariants

1. **Guest vs Host Endianness**:
   - Motorola 68000 is **strictly Big-Endian**; modern host machines are Little-Endian.
   - **Never** perform host-endian pointer casting or `transmute` on guest memory buffers.
   - Decode/encode multi-byte values using explicit endian conversion helpers (`u16::from_be_bytes`, `u32::from_be_bytes`, `val.to_be_bytes()`).
   - *Endianness Bypass*: Bitwise operations (`AND`, `OR`, `EOR`, `NOT`), zeroing (`CLR`), and memory block transfers (DMA, `MOVEM`, `MOVE (An), (Am)`) commute with byte reversal and can bypass endian swapping in hot paths.

2. **Zero Host Panics on Guest Code**:
   - Emulated guest code must **never panic the host process**. Zero `.unwrap()` / `.expect()` in runtime emulation paths.
   - Unmapped/disconnected address reads simulate open bus: return `$FF` for byte, `$FFFF` for word (standard A500 floating bus pulled high). Configurable via `MemoryBus::set_unmapped_byte()` for synthetic test harnesses.
   - Unaligned word/long accesses must trigger M68000 Address Error exception (Vector 3).

3. **Arithmetic & Overflow Handling**:
   - In debug builds, Rust panics on integer overflow. In emulation logic, ALU operations and cycle counters must explicitly use wrapping arithmetic (`wrapping_add`, `wrapping_sub`).

4. **Zero-Allocation Hot Path & WASM Constraints**:
   - Hot execution paths (`step()`, `step_cck()`, memory accesses, interrupt polling) must perform **zero dynamic heap allocations** (`Vec::new`, `Box::new`, `format!`, `String`). Use fixed arrays, bitflags, or in-place state.
   - Core crate constraints: No `std::time::Instant::now()`, no `std::thread`, no `std::fs` (load ROMs/disks as `&[u8]` byte slices).

5. **Subsystem Guidelines & Platform Quirks**:
   - Subsystem-specific rules (file sizes, canonical micro-steps, inlining, workspace layout) are modularized under `.agents/rules/` per Section 1.
   - Comprehensive silicon traps, non-intuitive timings, and anti-tamper invariants reside in [Platform Quirks and Invariants Catalog](Obsidian/Amiga/Design/Platform%20Quirks%20and%20Invariants%20Catalog.md).

---

## 4. Quality Assurance & Definition of Done

- **Mandatory Formatting:** `cargo fmt --all -- --check`.
- **Pre-Flight Gate:** Run `python tools/harness/pre_flight.py` (checks formatting, size, and architecture rules).
- **Automated Architecture Tests:** Pass `cargo test -p test_runner --test test_architecture_rules` (file sizes, zero panics/macros/generics, IDLE micro-steps, zero inline tests, inlining, links).
- **Single-Step CPU Validation:** Run `$env:SINGLESTEP_FULL = "1"; cargo test -p test_runner --test test_singlestep` on any `crates/m68000` changes.
- **Cartesian DMA Contention:** Run `cargo test -p test_runner --test test_dma_cartesian` on CPU/bus changes ($C = C_0 + 2 \times \text{wait\_states}$).
- **Repro-First Defect Resolution:** Author an isolated failing reproduction test in `tests/` before editing production code per [`repro-first.md`](.agents/rules/repro-first.md).
- **Unit Testing Policy:** Mandatory unit test coverage for functional/utility logic and headless integration tests for GUI per [`unit-testing-policy.md`](.agents/rules/unit-testing-policy.md).
- **Obsidian Design Docs:** Update design documents in [Obsidian/Amiga/Design](Obsidian/Amiga/Design) per [`docs-maintenance.md`](.agents/rules/docs-maintenance.md) and [`vault-linking-and-graph-integrity.md`](.agents/rules/vault-linking-and-graph-integrity.md).
- **Engineering Diary:** Log changes and rationale in [DIARY.md](DIARY.md) (Section 10) per [`diary-maintenance.md`](.agents/rules/diary-maintenance.md).
- **Roadmap Maintenance:** Prune completed tasks from [ROADMAP.md](ROADMAP.md) (zero completed items retained) per [`roadmap-maintenance.md`](.agents/rules/roadmap-maintenance.md).
- **Milestone Gates:** Run [`compact-diary`](.agents/skills/compact-diary/SKILL.md) and [`prune-dead-code`](.agents/skills/prune-dead-code/SKILL.md) upon major milestone completion.
- **Prohibition of Blind Golden Hash Modifications**: Zero silent edits to golden hashes/constants per [`spec-compliance.md`](.agents/rules/spec-compliance.md).
- **Milestone Review:** Run [`/code-review`](.agents/workflows/code-review.md) before declaring roadmap milestones complete.

---

## 5. Knowledge Base & Reference Navigation

- **Knowledge Retrieval Precedence**: Mandatory `graphify query` before viewing source files and MCP `rag_search` before opening reference manuals per [`graphify.md`](.agents/rules/graphify.md) and [`amiga-rag.md`](.agents/rules/amiga-rag.md).
- **Design Specifications**: Consult markdown documents under [Obsidian/Amiga/Design](Obsidian/Amiga/Design).
- **Platform Quirks & Invariants**: Centralized hardware silicon idiosyncrasies reside in [Platform Quirks and Invariants Catalog](Obsidian/Amiga/Design/Platform%20Quirks%20and%20Invariants%20Catalog.md).
- **Official Hardware Documentation**: Hardware manuals and PRMs reside under [Obsidian/Amiga/Reference](Obsidian/Amiga/Reference) (searchable via `rag_search`).
- **RAG Tooling & Infrastructure**: Amiga-focused FastMCP server in [`tools/amiga-rag-mcp-server`](tools/amiga-rag-mcp-server), backed by the shared `projects_docs` collection. Its `rag_reindex` tool indexes the Amiga documentation scopes through the PATH-resolved `rag_qdrant` command.
- **Reference Emulator Source Code**: Reference emulator (vAmiga) and test suite (vAmigaTS) in [ref_src](ref_src).
- **Single-Step Test Vectors**: M68000 silicon vectors in [ref_src/SingleStepTests-680x0/68000/v1](ref_src/SingleStepTests-680x0/68000/v1).


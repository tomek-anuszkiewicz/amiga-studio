# Amiga 500 Emulator Architectural & Engineering Guidelines

This repository contains the cycle-exact Amiga 500 emulator written in Rust.
All agentic pair-programming and automated modifications must adhere strictly to the architectural constraints, execution model, and coding guidelines detailed below.

---

## 1. Operating & Behavioral Rules (`.agents/rules/`)

Operational rules are modularized under `.agents/rules/` with single-responsibility scoping across two tiers:

### A. Universal Invariants (`trigger: always_on`)
- **Audio Voice Transcription** ([`audio-transcription.md`](.agents/rules/audio-transcription.md)): Mandatory spoken language transcript echo before responses.
- **Language Policy** ([`language-policy.md`](.agents/rules/language-policy.md)): Strict English for all agent responses, plans, artifacts, source code, and commit messages.
- **Dynamic Model Advisory** ([`model-reasoning-advisory.md`](.agents/rules/model-reasoning-advisory.md)): Proactive advice on switching between `Medium` and `High`/`Pro` reasoning.
- **Strict Path Privacy** ([`no-external-paths.md`](.agents/rules/no-external-paths.md)): Zero external host paths; use generic placeholders.
- **Specification Compliance** ([`spec-compliance.md`](.agents/rules/spec-compliance.md)): Zero silent divergence; mandatory user conflict escalation before code changes.
- **Hardware Efficiency & Readability** ([`performance-and-readability.md`](.agents/rules/performance-and-readability.md)): Flat execution, zero macros, zero const-generics, contiguous execution, and zero runtime heap allocations in hot paths.
- **Amiga RAG Knowledge Base** ([`amiga-rag.md`](.agents/rules/amiga-rag.md)): Pre-task conceptual retrieval (`source = "obsidian"`), vector search, and reference reindexing.
- **AGENTS.md Size & Limits** ([`agents-md-limits.md`](.agents/rules/agents-md-limits.md)): Strict constitutional size ceiling ($\le 14,000$ bytes) and non-redundancy policy.
- **Attractor & Vocabulary Discipline** ([`attractor-discipline.md`](.agents/rules/attractor-discipline.md)): Strict prevention of synthetic academic jargon monoculture and leaked hardware buzzwords.

### B. Domain-Specific Rules (`trigger: model_decision`)
- **Design Docs Maintenance** ([`docs-maintenance.md`](.agents/rules/docs-maintenance.md)): Synchronizing `Obsidian/Amiga/Design/` specs with code and pruning draft proposals.
- **Engineering Diary** ([`diary-maintenance.md`](.agents/rules/diary-maintenance.md)): Mandatory chronological narrative logging in `DIARY.md` (Section 10).
- **Roadmap Maintenance** ([`roadmap-maintenance.md`](.agents/rules/roadmap-maintenance.md)): Pruning completed tasks, updating baselines, and milestone gates in `ROADMAP.md`.
- **Vault Linking & Graph Integrity** ([`vault-linking-and-graph-integrity.md`](.agents/rules/vault-linking-and-graph-integrity.md)): Line 1 YAML properties evaluation, dual-layer linking, and zero broken links across `Obsidian/Amiga/Design/`.
- **Source File Size & Cohesion** ([`file-size-and-cohesion.md`](.agents/rules/file-size-and-cohesion.md)): File size $\le 800$ lines in `crates/*/src/` (with recognized exceptions), single responsibility, and flat instructions hierarchy.
- **Opcode Naming & Micro-Steps** ([`opcode-naming.md`](.agents/rules/opcode-naming.md)): Canonical `IDLE` micro-steps, 1:1 opcode files, and dual staging registers (`addr1`/`addr2`).
- **Method Inlining Strategy** ([`method-inlining.md`](.agents/rules/method-inlining.md)): Targeted `#[inline]`, `#[inline(always)]`, and `#[inline(never)]` annotations.
- **Workspace Architecture & Re-Exports** ([`workspace-structure-and-reexports.md`](.agents/rules/workspace-structure-and-reexports.md)): Strictly flat crate layout and 3-tier re-export hierarchy.
- **Rust Best Practices** ([`rust-best-practices.md`](.agents/rules/rust-best-practices.md)): Safe borrowing, zero unwraps in runtime, wrapping math, and unit tests.
- **Unit Testing Policy** ([`unit-testing-policy.md`](.agents/rules/unit-testing-policy.md)): Mandatory unit test coverage for functional/utility logic; headless integration tests for GUI.
- **egui & Frontend Best Practices** ([`egui-best-practices.md`](.agents/rules/egui-best-practices.md)): Synchronous state pull, 1:1 layout mapping, bounded time-slicing, and WASM/DPI adaptation.
- **Git Commits & Atomic History** ([`git-commits.md`](.agents/rules/git-commits.md)): Context reconstruction, atomic commit decomposition, Conventional Commits, and pre-commit test gates.
- **Git Merge Commits & Worktrees** ([`git-merge-commits.md`](.agents/rules/git-merge-commits.md)): Mandatory merge commits on conflict resolution; worktree lifecycle and cleanup.
- **Graphify Knowledge Graph** ([`graphify.md`](.agents/rules/graphify.md)): Architecture queries and AST relationships; scoped incremental updates for `crates/` and `ref_src/`.
- **Asset Descriptions & Sidecars** ([`asset-descriptions.md`](.agents/rules/asset-descriptions.md)): Git-tracked `<image_path>.txt` technical sidecars for circuit and timing schematics.
- **Parallel Execution & Async Tasks** ([`parallel-execution.md`](.agents/rules/parallel-execution.md)): Non-blocking background tasks, targeted sub-suite testing, and multi-agent workflows.

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
   - Top-level machine struct (`A500`) owns all major subsystems: `Cpu` (`M68000`), `MemoryBus`, `CycleCounter`, `Agnus`, `Denise`, `Paula`, `CiaA`, and `CiaB`.
   - **No circular references**: Subsystems must not hold direct pointers or circular handles (`Rc<RefCell<...>>`).
   - All chip coordination, interrupt priority line (IPL 1-6) arbitration, and bus locks are driven in the main machine loop and `MemoryBus`.

4. **Hardware Circuit Simulation & Save States**:
   - Physical circuit simulation: register modifications and bus signals take effect on subsequent clock phases or cycles rather than propagating instantaneously.
   - Subsystem state structs (`CpuState`, custom chip states) are decoupled from runtime handles, fully queryable (read-only snapshots), and implement `serde::Serialize` and `serde::Deserialize`.

---

## 3. Rust Systems & Machine Invariants

1. **Guest vs Host Endianness**:
   - Motorola 68000 is **strictly Big-Endian**; modern host machines are Little-Endian.
   - **Never** perform host-endian pointer casting or `transmute` on guest memory buffers.
   - Always decode/encode multi-byte values using explicit endian conversion helpers (`u16::from_be_bytes`, `u32::from_be_bytes`, `val.to_be_bytes()`).
   - *Endianness Bypass*: Bitwise operations (`AND`, `OR`, `EOR`, `NOT`), zeroing (`CLR`), and memory block transfers (DMA, `MOVEM`, `MOVE (An), (Am)`) commute with byte reversal and can bypass endian swapping in performance-critical hot paths.

2. **Zero Host Panics on Guest Code**:
   - Emulated guest code must **never panic the host process**. Do not use `.unwrap()` or `.expect()` in runtime emulation paths.
   - Unmapped/disconnected address reads simulate open bus: return `$FF` for byte, `$FFFF` for word (standard A500 floating bus pulled high). Configurable via `MemoryBus::set_unmapped_byte()` for synthetic test harnesses (e.g. SingleStepTests flat memory defaulting to `$00`).
   - Unaligned word/long accesses must trigger M68000 Address Error exception (Vector 3).

3. **Arithmetic & Overflow Handling**:
   - In debug builds, Rust panics on integer overflow. In emulation logic, ALU operations and cycle counters must explicitly use wrapping arithmetic (`wrapping_add`, `wrapping_sub`).

4. **Zero-Allocation Hot Path & WASM Constraints**:
   - Hot execution paths (`step()`, `step_cck()`, memory accesses, interrupt polling) must perform **zero dynamic heap allocations** (`Vec::new`, `Box::new`, `format!`, `String`). Use fixed arrays, bitflags, or in-place state.
   - Core crate constraints: No `std::time::Instant::now()` (panics in WASM without shims), no `std::thread`, no `std::fs` (load ROMs/disks as `&[u8]` byte slices).

5. **Subsystem Architecture & Implementation Guidelines**:
   Detailed operational and architectural guidelines are modularized under `.agents/rules/` to avoid redundancy:
   - **Hardware Efficiency & Prohibited Patterns:** Flat execution, zero `macro_rules!`, zero const-generic instruction handlers, and out-of-line cold exceptions $\rightarrow$ [`performance-and-readability.md`](.agents/rules/performance-and-readability.md).
   - **File Size & Cohesion:** Strict $\le 800$-line threshold for `.rs` files and 1:1 opcode-to-file mapping under `crates/m68000/src/instructions/` with zero subdirectories $\rightarrow$ [`file-size-and-cohesion.md`](.agents/rules/file-size-and-cohesion.md).
   - **Method Inlining:** Targeted rules for `#[inline]`, `#[inline(always)]`, and `#[inline(never)]` $\rightarrow$ [`method-inlining.md`](.agents/rules/method-inlining.md).
   - **Workspace Structure:** Flat crates directory and 3-tier re-export hierarchy $\rightarrow$ [`workspace-structure-and-reexports.md`](.agents/rules/workspace-structure-and-reexports.md).
   - **Opcode Micro-Steps & Dual Staging:** Mandatory `IDLE` micro-step naming and dual staging registers (`addr1`, `addr2`) with split address error invariance $\rightarrow$ [`opcode-naming.md`](.agents/rules/opcode-naming.md) and [`performance-and-readability.md`](.agents/rules/performance-and-readability.md).

---

## 4. Quality Assurance & Definition of Done

- **Mandatory Formatting:** `cargo fmt --all -- --check`.
- **Automated Architecture Tests:** Pass `cargo test -p test_runner --test test_architecture_rules` (validates file sizes, zero runtime panics, zero custom macros, zero const generics, canonical IDLE steps, path privacy, inlining, and link integrity).
- **Single-Step CPU Validation:** On any changes to `crates/m68000`:
  ```powershell
  $env:SINGLESTEP_FULL = "1"; cargo test -p test_runner --test test_singlestep
  ```
  Validates all instruction test suites against Tom Harte physical silicon vectors in parallel.
- **Cartesian DMA Contention:** On CPU or bus changes:
  ```powershell
  cargo test -p test_runner --test test_dma_cartesian
  ```
  Validates cycle invariance ($C = C_0 + 2 \times \text{wait\_states}$), Fast RAM immunity, and state invariance across $2^k \times 2^M$ permutation space.
- **Unit Testing Policy:** Mandatory unit test coverage for functional/utility logic and headless integration tests for GUI per [`unit-testing-policy.md`](.agents/rules/unit-testing-policy.md).
- **Obsidian Design Docs:** Update corresponding design documents in [Obsidian/Amiga/Design](Obsidian/Amiga/Design) per [`docs-maintenance.md`](.agents/rules/docs-maintenance.md) and evaluate Line 1 YAML properties per [`vault-linking-and-graph-integrity.md`](.agents/rules/vault-linking-and-graph-integrity.md).
- **Engineering Diary:** Log actual changes, technical rationale, and test results in [DIARY.md](DIARY.md) (Section 10) per [`docs-maintenance.md`](.agents/rules/docs-maintenance.md).
- **Roadmap Maintenance:** Mark completed tasks and prune active list in [ROADMAP.md](ROADMAP.md) per [`docs-maintenance.md`](.agents/rules/docs-maintenance.md).
- **Milestone Gates:** Run [`compact-diary`](.agents/skills/compact-diary/SKILL.md) and [`prune-dead-code`](.agents/skills/prune-dead-code/SKILL.md) upon major roadmap milestone completion.
- **Prohibition of Blind Golden Hash Modifications**: Modifying golden test hashes, cycle totals, or benchmark reference constants to silence a failing test is strictly forbidden per [`spec-compliance.md`](.agents/rules/spec-compliance.md). Perform root-cause analysis on regressions.
- **Milestone Review:** Run [`/code-review`](.agents/workflows/code-review.md) before declaring roadmap milestones complete.

---

## 5. Knowledge Base & Reference Navigation

- **Design Specifications**: Consult markdown documents under [Obsidian/Amiga/Design](Obsidian/Amiga/Design).
- **Official Hardware Documentation**: Amiga Hardware Reference Manual, 68000 PRMs, and Guru Book reside under [Obsidian/Amiga/Reference](Obsidian/Amiga/Reference) and can be searched via `rag_search` tool (`amiga-rag`).
- **RAG Tooling & Infrastructure**: Pipeline, CLI indexer (`amiga_rag`), and FastMCP server reside in [`tools/rag`](tools/rag), backed by local Qdrant vector database (`amiga` collection).
- **Reference Emulator Source Code**: Clean-room reference implementation (vAmiga) and test suite (vAmigaTS) reside in [ref_src](ref_src).
- **Single-Step Test Vectors**: Official physical silicon test vectors for M68000 CPU are in [ref_src/SingleStepTests-680x0/68000/v1](ref_src/SingleStepTests-680x0/68000/v1).

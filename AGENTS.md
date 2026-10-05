# Amiga 500 Emulator Architectural & Engineering Guidelines

All agent work on this cycle-exact Rust Amiga 500 emulator must follow these architectural and engineering guidelines.

---

## 1. Operating & Behavioral Rules (`.agents/rules/`)

Codex loads this file as project instructions. Read every universal rule before task work and relevant domain rules before working in that domain. Linked rule files require explicit reads.

### A. Universal Rules (read for every task)
- **[Audio Voice Transcription](.agents/rules/audio-transcription.md)**: Echo spoken prompts in their language.
- **[Dynamic Model Advisory](.agents/rules/model-reasoning-advisory.md)**: Advise on reasoning effort.
- **[Strict Path Privacy](.agents/rules/no-external-paths.md)**: Use portable paths and placeholders.
- **[Specification Compliance](.agents/rules/spec-compliance.md)**: Escalate specification conflicts.
- **[Hardware Efficiency & Readability](.agents/rules/performance-and-readability.md)**: Flat, allocation-free hot paths; no macros/const-generics.
- **[Amiga RAG Knowledge Base](.agents/rules/amiga-rag.md)**: Pre-task retrieval and CLI indexing/search.
- **[Unit Testing Policy](.agents/rules/unit-testing-policy.md)**: Dedicated `tests/`; no inline tests in `src/`.
- **[Immediate Atomic Commits](.agents/rules/git-commits.md)**: Atomic Conventional Commits.
- **[Structural Root-Cause Resolution](.agents/rules/structural-root-cause.md)**: Fix upstream mechanisms; no symptom patches.
- **[Strict Scope Discipline](.agents/rules/strict-scope-discipline.md)**: Minimal diffs and task containment.
- **[Prime Directives](.agents/rules/prime-directives.md)**: Model invariants, reuse existing definitions, and contain edits to requested work.
- **[Clean-Break Refactoring](.agents/rules/clean-break-refactoring.md)**: Complete cutover; no legacy shims.

### B. Domain Rules (read when relevant)
- **[Language Policy](.agents/rules/language-policy.md)**: User's language in chat; English in repository artifacts.
- **[Hardware Bus Topology](.agents/rules/hardware-bus-topology.md)**: Agnus DMA address mastership, passive latching, zero inter-chip signal smuggling. Continuous + Per-Commit: AST verified in `pre_flight.py --quick` (Hardware Quality Pillar 1 & 2).
- **[Design Docs Maintenance](.agents/rules/docs-maintenance.md)**: Sync `Obsidian/Amiga/Design/` specs with code; prune draft proposals.
- **[Diary Maintenance](.agents/rules/diary-maintenance.md)**: Log and compact completed milestones.
- **[Roadmap Maintenance](.agents/rules/roadmap-maintenance.md)**: Substrate-first ordering, zero completed items retention in `ROADMAP.md`.
- **[Information Hierarchy & Limits](.agents/rules/information-hierarchy.md)**: Top-down structure; AGENTS.md $\le 14,000$ bytes.
- **[Vault Linking & Graph Integrity](.agents/rules/vault-linking-and-graph-integrity.md)**: Line 1 YAML properties, dual-layer linking, zero broken links.
- **[Source File Size & Cohesion](.agents/rules/file-size-and-cohesion.md)**: Source files $\le 800$ lines in `crates/*/src/`, single responsibility.
- **[Opcode Naming & Micro-Steps](.agents/rules/opcode-naming.md)**: `IDLE` steps, 1:1 files, `addr1`/`addr2` staging.
- **[Method Inlining Strategy](.agents/rules/method-inlining.md)**: Targeted inlining annotations.
- **[Workspace Architecture & Re-Exports](.agents/rules/workspace-structure-and-reexports.md)**: Flat crates and 3-tier re-exports.
- **[Rust Best Practices](.agents/rules/rust-best-practices.md)**: Safe borrowing, zero runtime unwraps, wrapping math, minimum visibility.
- **[Repro-First Defect Resolution](.agents/rules/repro-first.md)**: Mandatory isolated failing test before modifying production code.
- **[egui & Frontend Best Practices](.agents/rules/egui-best-practices.md)**: Synchronous state pull, bounded time-slicing, WASM/DPI adaptation.
- **[Git Merge Commits & Worktrees](.agents/rules/git-merge-commits.md)**: Mandatory merge commits on conflict resolution; worktree lifecycle.
- **[Graphify Knowledge Graph](.agents/skills/graphify/SKILL.md)**: Read the skill before code navigation and graph refresh work.
- **[Parallel Execution & Async Tasks](.agents/rules/parallel-execution.md)**: Non-blocking background tasks, targeted sub-suite testing.
- **[Practitioner Voice & Tone](.agents/rules/practitioner-voice-and-tone.md)**: Practical engineering prose.

### Codex Harness

- Skills: `.agents/skills/*/SKILL.md`; invoke with `$skill-name` or select through `/skills`.
- Custom agents: `.codex/agents/*.toml`; delegate when requested by the user or applicable instructions.
- MCP: `.codex/config.toml`.
- Run validation commands explicitly. Do not install, configure, or use lifecycle, code, or Git hooks.

### Reasoning Before Scripts

- First identify the question to resolve and whether reasoning from the available evidence is sufficient.
- Use a script when a concrete measurement, verification, or data-processing step is needed. A matching filename alone does not justify using it.
- Treat temporary files in `.agent/tmp/` as artifacts of earlier attempts whose assumptions need checking before reuse.
- Before repairing or extending a tool, reassess whether that work is necessary for the user's task. If it is not, return to the original question and choose a simpler approach.

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
   - **No circular references & zero signal smuggling**: Subsystems must not hold direct pointers or call peer methods directly.
   - **Agnus DMA Address Mastership**: Agnus exclusively drives Chip RAM DMA addresses and RGA bus lines; Denise and Paula are passive data latchers with zero direct memory reads.
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
   - Unmapped reads return `$FF` / `$FFFF` (floating open bus pulled high; configurable via `set_unmapped_byte()`).
   - Unaligned word/long accesses must trigger M68000 Address Error exception (Vector 3).

3. **Arithmetic & Overflow Handling**:
   - In emulation logic, ALU operations and cycle counters must explicitly use wrapping arithmetic (`wrapping_add`, `wrapping_sub`).

4. **Zero-Allocation Hot Path & WASM Constraints**:
   - Hot paths (`step()`, `step_cck()`, memory accesses, interrupt polling) must perform **zero dynamic heap allocations** (`Vec`, `Box`, `format!`, `String`).
   - Core crate constraints: No `std::time::Instant::now()`, no `std::thread`, no `std::fs` (load ROMs/disks as byte slices).

5. **Subsystem Guidelines & Platform Quirks**:
   - Subsystem-specific rules (file sizes, canonical micro-steps, inlining, workspace layout) are modularized under `.agents/rules/` per Section 1.
   - Comprehensive silicon traps, non-intuitive timings, and anti-tamper invariants reside in [Platform Quirks and Invariants Catalog](Obsidian/Amiga/Design/Platform%20Quirks%20and%20Invariants%20Catalog.md).

---

## 4. Quality Assurance & Definition of Done

- **Per-Commit Gate (Routine Micro-Commits):**
  - Quick gate: `python tools/harness/pre_flight.py --quick`.
  - Automated architecture tests: `cargo test -p test_runner --test test_architecture_rules` *(not included in `--quick`; must be run separately)*.
  - Atomic commit following Conventional Commits in strict English per [`git-commits.md`](.agents/rules/git-commits.md).
  - *Routine micro-commits must not add diary entries; design doc synchronization and roadmap pruning occur at minor roadmap points or milestones.*
- **Minor Roadmap Point & Milestone Gates (Step 1.1, 1.2, 2.1...):**
  - Milestone pre-flight: `python tools/harness/pre_flight.py --milestone` (Code Quality 3-4, Hardware Quality 3-5, Docs Quality 100%).
  - Semantic parity: run [`audit-semantic-parity`](.agents/skills/audit-semantic-parity/SKILL.md) on modified subsystems.
  - Design docs sync & checkpoint bump per [`docs-maintenance.md`](.agents/rules/docs-maintenance.md).
  - Log milestone completion in [DIARY.md](DIARY.md) (Section 10) via `tools/harness/log_diary.py` and run [`compact-diary`](.agents/skills/compact-diary/SKILL.md) on minor points or major phase completions.
  - Prune completed steps from [ROADMAP.md](ROADMAP.md) (zero retention) per [`roadmap-maintenance.md`](.agents/rules/roadmap-maintenance.md).
- **Verification Suites & Invariants:**
  - Single-Step CPU Validation: Run `$env:SINGLESTEP_FULL = "1"; cargo test -p test_runner --test test_singlestep` on any `crates/cpu` changes.
  - Cartesian DMA Contention: Run `cargo test -p test_runner --test test_dma_cartesian` on CPU/bus changes ($C = C_0 + 2 \times \text{wait\_states}$).
  - Repro-First Defect Resolution: Author an isolated failing reproduction test in `tests/` before editing production code per [`repro-first.md`](.agents/rules/repro-first.md).
  - Prohibition of Blind Golden Hash Modifications: Zero silent edits to golden hashes/constants per [`spec-compliance.md`](.agents/rules/spec-compliance.md).

---

## 5. Knowledge Base & Reference Navigation

- **Knowledge Retrieval Precedence**: Follow the [Graphify skill](.agents/skills/graphify/SKILL.md) for source navigation and [Amiga RAG rule](.agents/rules/amiga-rag.md) for reference retrieval.
- **Design Specifications**: Consult markdown documents under [Obsidian/Amiga/Design](Obsidian/Amiga/Design).
- **Platform Quirks & Invariants**: Centralized hardware silicon idiosyncrasies reside in [Platform Quirks and Invariants Catalog](Obsidian/Amiga/Design/Platform%20Quirks%20and%20Invariants%20Catalog.md).
- **Official Hardware Documentation**: Hardware manuals and PRMs reside under [Obsidian/Amiga/Reference](Obsidian/Amiga/Reference) (searchable via `rag_search`).
- **RAG Tooling & Infrastructure**: Canonical `rag_qdrant` CLI plus the optional FastMCP adapter in [`tools/amiga-rag-mcp-server`](tools/amiga-rag-mcp-server), backed by Qdrant (`projects_docs` collection, `amiga` source).
- **Reference Emulator Source Code**: Reference emulator (vAmiga) and test suite (vAmigaTS) in [ref_src](ref_src).
- **Single-Step Test Vectors**: M68000 silicon vectors in [ref_src/SingleStepTests-680x0/68000/v1](ref_src/SingleStepTests-680x0/68000/v1).

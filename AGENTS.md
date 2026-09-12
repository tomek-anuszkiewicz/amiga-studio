# Amiga 500 Emulator Architectural & Engineering Guidelines

This repository contains the cycle-exact Amiga 500 emulator written in Rust.
All agentic pair-programming and automated modifications must adhere strictly to the architectural constraints, execution model, and coding guidelines detailed below.

---

## 1. Operating & Behavioral Rules (`.agents/rules/`)

Operational, communication, and interaction rules are modularized under `.agents/rules/` with `always_on` enforcement:
- **Audio Voice Transcription** ([`audio-transcription.md`](.agents/rules/audio-transcription.md)): Mandatory spoken language transcript echo before responses.
- **Language Policy** ([`language-policy.md`](.agents/rules/language-policy.md)): Strict English for all agent responses, plans, artifacts, source code, and commit messages.
- **Dynamic Model Advisory** ([`model-reasoning-advisory.md`](.agents/rules/model-reasoning-advisory.md)): Proactive advice on switching between `Medium` and `High`/`Pro` reasoning.
- **Strict Path Privacy** ([`no-external-paths.md`](.agents/rules/no-external-paths.md)): Zero external host paths; use generic placeholders.
- **Specification Compliance** ([`spec-compliance.md`](.agents/rules/spec-compliance.md)): Zero silent divergence; mandatory user conflict escalation before code changes.
- **Mechanical Sympathy & Readability** ([`performance-and-readability.md`](.agents/rules/performance-and-readability.md)): Flat execution, zero macros, zero const-generics, cache density, and zero runtime heap allocations in hot paths.
- **Parallel Execution & Async Tasks** ([`parallel-execution.md`](.agents/rules/parallel-execution.md)): Non-blocking background tasks, targeted sub-suite testing, and multi-agent workflows.
- **Graphify Knowledge Graph** ([`graphify.md`](.agents/rules/graphify.md)): Architecture queries and AST relationships via graphify.
- **Rust Best Practices** ([`rust-best-practices.md`](.agents/rules/rust-best-practices.md)): Safe borrowing, zero unwraps in runtime, wrapping math, no macros/const generics, and mandatory unit tests for all testable logic.
- **egui & Frontend Best Practices** ([`egui-best-practices.md`](.agents/rules/egui-best-practices.md)): Synchronous state pull, 1:1 layout mapping, bounded time-slicing, and WASM/DPI adaptation.
- **Unit Testing Policy** ([`unit-testing-policy.md`](.agents/rules/unit-testing-policy.md)): Mandatory unit test coverage for all functional/utility classes; headless integration tests for GUI.
- **Git Merge Commits & Worktrees** ([`git-merge-commits.md`](.agents/rules/git-merge-commits.md)): Mandatory merge commits on conflict resolution; worktree lifecycle and cleanup.

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

## 3. Rust Systems & Emulator Coding Guidelines

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

5. **Host CPU Mechanical Sympathy: Branch Prediction, Cache Density & Zero Readability Compromise**:
   - **Deep Pipelines & Flat Execution**: Eliminate cascaded runtime branches (`match opcode`, `match ea_mode`, `if size == Size::Byte`) in the hot loop. The core favors **direct, flattened code flows** (e.g. 65,536-entry static dispatch table `[fn; 65536]`, specialized opcode handlers) where size, addressing mode, and registers are statically baked in. Code may be expansive and unrolled if it eliminates dynamic branching in the hot path.
   - **L1i Cache Density**: Keep hot instruction dispatch compact. Mark heavy, rarely taken exception handling (Address Error 7-word frame synthesis, illegal instruction traps, bus fault diagnostics) with `#[inline(never)]` so cold error recovery never pollutes hot L1i cache lines.
   - **Readability Without Compromise**: High performance must never be an excuse for unreadable code or cryptic tricks. Code must remain clean, modular, self-documenting, and idiomatic Rust.
   - **Strict Prohibition of User-Defined Macros (`macro_rules!` is Forbidden)**: Custom macros (`macro_rules!`) are strictly forbidden across the codebase. Macros break IDE code navigation, obscure call sites, produce opaque compiler errors, and add cognitive complexity. Repetitive code, static dispatch tables, and handlers must be written as explicit, self-documenting Rust functions, direct calls, or compile-time `const fn` arrays.
   - **Prohibition of Const-Generic Functions with Constant Parameters**: Using generic functions where generic parameters are constants (`fn op_foo<const S: usize, const M: usize>(...)`) is forbidden for instruction handlers, decoding logic, and core execution paths. Const generics obscure concrete execution paths, fragment IDE navigation, and complicate debugging. Handlers and execution logic must be authored as explicit, concrete, specialized Rust functions or direct flattened control flows.

6. **Module Cohesion & Rust Source File Size Guidelines (`.rs` Files Only)**:
   - **Strict Scope: Rust Source Code Files Only (`.rs`)**: The 800-line threshold applies strictly to Rust source code files in `crates/*/src/`. Technical documentation and reference manuals under `Obsidian/` and docs directories have **no line count limits**.
   - **Cohesion over Arbitrary Fragmentation**: Group closely related structs, enums, and handlers in the same file when they cover the same architectural aspect (e.g. `MemoryBank`, `BankHandler`, and bank functions in `map.rs`).
   - **Rust Source Size Thresholds**:
     - *< 300 lines*: Healthy baseline for single-aspect modules and state structures.
     - *300–600 lines*: Ideal sweet spot for cohesive units combining types, enums, and operational logic.
     - *600–800 lines*: Review trigger. Review for multiple responsibilities (SRP violation) or separable test code.
     - *> 800 lines*: Split mandate. Rust source files exceeding 800 lines must be split into submodules unless they meet the criteria for a Recognized Exception.
   - **Strict Flat Instruction Hierarchy & 1:1 Mnemonic Mapping (`crates/m68000/src/instructions/`)**:
     - **1:1 Mnemonic-to-File Principle**: Every distinct M68000 CPU instruction mnemonic must reside in its own dedicated flat Rust file directly under `crates/m68000/src/instructions/<mnemonic>.rs` (e.g. `mulu.rs`, `muls.rs`, `divu.rs`, `divs.rs`, `link.rs`, `unlk.rs`, `abcd.rs`, `sbcd.rs`, `nbcd.rs`, `trapv.rs`, `rtr.rs`, `rte.rs`, `stop.rs`, `reset.rs`, `move_usp.rs`, `add.rs`, `sub.rs`, `and.rs`, `or.rs`, `cmpi.rs`).
     - **Prohibition of Umbrella Files & Subdirectories**: Grouping disparate instruction mnemonics into umbrella files (e.g. `mul.rs`, `div.rs`, `bcd.rs`) is strictly forbidden. Creating subdirectories under `crates/m68000/src/instructions/` (such as `and/`, `cmpi/`) is **strictly forbidden** (directory must remain strictly flat).
     - **Recognized Exceptions**:
       - Coupled SR/CCR operations: `move_sr_ccr.rs` (`MOVE from/to SR/CCR`) and `logic_sr_ccr.rs` (`ANDI/EORI/ORI to CCR/SR`).
       - Size-based decompositions for high-cardinality operations: `move_b.rs`, `move_w.rs`, `move_l.rs`.
       - Files registered in `LINE_COUNT_EXCEPTIONS` in `test_architecture_rules.rs`: static dispatch tables (`dispatch_table.rs`), disassembler (`disassembler.rs`), and exhaustive linear decoders (`add.rs`, `sub.rs`, `and.rs`, `or.rs`, `cmpi.rs`, `move_b.rs`, `move_w.rs`, `move_l.rs`). Never split into subdirectories.

7. **Method Inlining Strategy (`#[inline]`, `#[inline(always)]`, `#[inline(never)]`)**:
   - `#[inline]` emits intermediate representation into crate metadata, enabling **cross-crate inlining** across workspace crates without requiring whole-program LTO.
   - **Use `#[inline]` on**: Public getters, setters, single-expression accessors called across crates (`chip_ram()`, `is_chip_ram_locked()`), lightweight forwarding wrappers (`step_cck`), and endian conversion helpers.
   - **Use `#[inline(always)]` on**: Ultra-hot arithmetic/logic and CCR condition code flag calculations ($X, N, Z, V, C$) executed multiple times per CCK cycle.
   - **Avoid `#[inline]` on**: Functions with > 15–20 lines of complex control flow and indirect dispatch table targets (e.g. `BankHandler.read_byte` function pointers, 65,536-entry opcode handlers).
   - **Use `#[inline(never)]` on**: Cold exception paths, address error dumps, illegal instruction traps, and diagnostic panic paths to keep the hot dispatch loop contiguous in L1i cache.

8. **Workspace Flat Layout & 3-Tier Re-Export (`pub use`) Strategy**:
   - Keep crate directories in `crates/*` **strictly flat** (no nested crate folders).
     - **Tier 1 (Foundational Blueprint - `config`)**: Machine-wide presets/timings. Never re-exported by peer subsystems.
     - **Tier 2 (Peer Subsystems - `memory_bus`, `m68000`, `agnus`, `denise`, `paula`, `cia`)**: Peers owned by `A500`. Peers **never re-export other peers**.
     - **Tier 3 (Contained Sub-Components - `rtc`, `copper`, `blitter`)**: Conceptually owned by a specific subsystem. Parent peer **must** re-export them (`pub use rtc; pub use rtc::RtcMsm6242b;`).
     - **Tier 0 (Top-Level Facade - `a500` machine)**: Owns all peers and acts as the unified gateway for host frontends (`web-wasm`, `desktop-gui`, `cli`).

9. **Mandatory Idle Micro-Step Naming & Prohibition of Anonymous Idle Structs (`crates/m68000/src/instructions/`)**:
   - **Canonical Idle Constants**: Any M68000 micro-step where the memory bus performs no transfer (or internal ALU idle cycles) **must** explicitly feature `IDLE` in its identifier:
     - Bus idle phases: `common::BUS_READ_IDLE`, `common::BUS_WRITE_IDLE` (or stack/exception variants `PUSH_STACK_HIGH_IDLE`, `EXCEPTION_PUSH_*_IDLE`, `AERR_PUSH_*_IDLE`).
     - Internal ALU idle cycles: `common::ALU_IDLE` (2-clk), `common::ALU_IDLE_4CLK` (4-clk), `common::ALU_IDLE_8CLK` (8-clk), `common::ALU_IDLE_128CLK` (128-clk).
   - **Prohibition of Anonymous Idle Structs & Legacy Aliases**: Inlining raw struct literals like `MicroStep { step_fn: None, alu_fn: None, base_clocks: N }` inside instruction arrays or handlers is **strictly forbidden** (use canonical constants in `common::`). Misleading finish/retire aliases (e.g. `READ_WORD_FINISH`, `PREFETCH_NEXT_RETIRE`) are strictly prohibited.

10. **Dual Staging Architecture (`addr1`, `addr2`) & Split Address Error Invariance (`crates/m68000/`)**:
    - **Dual Staging Registers**: For dual-memory instructions (`CMPM`, `ABCD`, `SBCD`, `ADDX`, `SUBX`), stage effective addresses in `state.micro.addr1` ($X_1$, source) and `state.micro.addr2` ($X_2$, destination).
    - **Byte Operations Upfront Calculation**: For byte operations (`CMPM.b`, `ABCD`, `SBCD`, `ADDX.b`, `SUBX.b`), precalculate both `addr1` and `addr2` upfront (`ea_calc_dual_pi_b` / `ea_calc_dual_pd_b`). Byte transfers never fault on alignment.
    - **Word/Long Split Calculation & Address Error Invariance**: For word and long operations (`CMPM.w/l`, `ADDX.w/l`, `SUBX.w/l`), source address calculation must occur in Step 0, while destination address calculation must be deferred and fused into the **CCK2 idle phase of the source read**. If the source address is unaligned (odd), the CPU immediately triggers Vector 3 Address Error with the destination register ($Ax$, USP/SSP) completely untouched.
    - **Direct Staged Writes**: Memory writes must use dedicated `WRITE_ADDR2_*` blocks (`WRITE_ADDR2_BYTE`, `WRITE_ADDR2_WORD`, `WRITE_ADDR2_PD_LONG_LOW`, `WRITE_ADDR2_PD_LONG_HIGH`). Staging temporary pointers in `scratch[0..2]`, shifting `destination >>= 16`, or using pointer-swapping helpers is strictly forbidden.
    - **Fused CCK Operations**: Always fuse ALU/EA calculations onto natural 2-clock Color Clock phases (`MicroStep.alu_fn`) rather than introducing separate zero-clock micro-steps.

---

## 4. Documentation Maintenance & Quality Assurance (Definition of Done)

- **Mandatory Final Task:** Whenever an agent implements, refactors, or modifies a subsystem, **update the corresponding design document in [Obsidian/Amiga/Design](Obsidian/Amiga/Design)** if any architectural decision, timing model, data structure, or hardware quirk has changed or was clarified.
- **Roadmap Step Completion & Pruning:** When a roadmap milestone or step in [ROADMAP.md](ROADMAP.md) is 100% verified (all tests green), **update [ROADMAP.md](ROADMAP.md) as part of that same task/PR**: remove the completed task from the active list, update the completed baseline summary, and renumber/reorder remaining steps.
- **Design Document Pruning & Code Duplication Removal:** Review and clean up relevant design documents when completing features: remove obsolete speculative code, prune superseded draft proposals, and ensure documents reflect living reality. **Design documents must never duplicate code that has already been written**: replace duplicate Rust code blocks with concise architectural descriptions, tables, and direct markdown links to living Rust source files.
- **Crate Dependency Graph Maintenance:** When workspace dependencies in `Cargo.toml` change, update the Crate Dependency Mermaid Graph in [Obsidian/Amiga/Design/General Architecture.md](Obsidian/Amiga/Design/General%20Architecture.md#2-workspace-crate-architecture--dependencies).
- **Mandatory Formatting & Automated Architecture Tests:**
  - Run `cargo fmt --all` across workspace. All changes must pass `cargo fmt --all -- --check`.
  - Pass the automated architectural test suite:
    ```powershell
    cargo test -p test_runner --test test_architecture_rules
    ```
    (enforcing formatting, file size <= 800 lines in `crates/*/src/`, rule file size <= 23 KB in `AGENTS.md` and `.agents/rules/*.md`, flat instruction hierarchy with zero subdirectories, zero runtime panics/unwraps, zero custom macros, zero const-generic handlers, canonical idle micro-steps, path privacy, and inlining rules).
- **Mandatory Full SingleStepTests on M68000 Changes:**
  ```powershell
  $env:SINGLESTEP_FULL = "1"; cargo test -p test_runner --test test_singlestep
  ```
  Validates all ~300,000 test cases across MAME and Tom Harte hardware vectors in parallel. Tasks touching `crates/m68000` cannot be declared complete without this pass.
- **Mandatory Cartesian DMA Contention Verification on M68000 Changes:**
  ```powershell
  cargo test -p test_runner --test test_dma_cartesian
  ```
  Validates cycle invariance ($C = C_0 + 2 \times \text{wait\_states}$), Fast RAM immunity, and state invariance across $2^k \times 2^M$ permutation space.
- **Mandatory Defect Retrospection & Institutional Prevention (Blameless Root-Cause Analysis)**:
  Whenever fixing a bug, regression, or oversight:
  1. **Root-Cause Retrospection ("Why did this happen?")**: Perform structured self-retrospection on why the bug or oversight occurred.
  2. **Institutionalization ("How do we ensure this never repeats?")**:
     - Write dedicated regression and invariant tests.
     - Evaluate automated enforcement in `test_architecture_rules.rs`.
     - Update design documents in `Obsidian/Amiga/Design/` or rules in `AGENTS.md`.
     - Add explicit checkpoints to Definition of Done and `/code-review`.
- **Mandatory Comprehensive Unit Test Coverage for Testable Logic:** Every newly created or modified Rust source file containing testable domain logic, state machines, hardware models, math/ALU operations, algorithms, statistics, parsers, or program builders must have dedicated unit tests (either inline `#[cfg(test)] mod tests { ... }` or in dedicated test targets under `tests/<module_name>.rs`). A module must never be declared complete without tests covering happy paths, boundary conditions, zero/empty states, and failure modes.
- **Mandatory Post-Flight Compliance Checklist:** Conclude every implementation task with a Definition of Done checklist verifying compliance with systems rules (zero panics, wrapping math, inlining, zero custom macros, zero const-generic handlers, canonical idle micro-step naming, Rust source file size <= 800 lines, flat instruction hierarchy with zero subdirectories in `crates/m68000/src/instructions/`, strict English in all source code and comments, defect retrospection & regression coverage, comprehensive unit test coverage for all testable logic, design doc & roadmap pruning, removal of implemented code snippets from design docs, 100% green tests).
- **Prohibition of Blind Golden Hash Modifications**: Modifying golden test hashes, cycle totals, or benchmark reference constants (e.g. `GOLDEN_CATALOG_STRUCTURE_HASH`, `GOLDEN_CSV_HASH_*` in `test_benchmark_csv.rs`, or single-step test fixtures) to silence a failing test is strictly forbidden. A hash divergence signifies catalog, instruction encoding, or cycle timing regression. When a test fails, perform root-cause analysis on the implementation. Golden hash modifications require formal hardware justification and explicit user escalation per `spec-compliance.md`.
- **Sub-Agent Milestone Review Protocol (`/code-review`):** Before declaring a roadmap milestone complete, invoke an independent review subagent or follow the `/code-review` workflow to audit the diff with a clean context before user hand-off.

---

## 5. Knowledge Base & Reference Navigation

- **Design Specifications**: Consult markdown documents under [Obsidian/Amiga/Design](Obsidian/Amiga/Design).
- **Official Hardware Documentation**: Amiga Hardware Reference Manual, 68000 PRMs, and Guru Book reside under [Obsidian/Amiga/Reference](Obsidian/Amiga/Reference) and can be searched via `rag_search` tool (`amiga-rag`).
- **RAG Tooling & Infrastructure**: Pipeline, CLI indexer (`amiga_rag`), and FastMCP server reside in [`tools/rag`](tools/rag), backed by local Qdrant vector database (`amiga` collection).
- **Reference Emulator Source Code**: Verified reference implementations (MAME, Moira, Musashi, vAmiga, WinUAE) are in [ref_src](ref_src).
- **Single-Step Test Vectors**: Official test suite for M68000 CPU is in [ref_src/SingleStepTests-m68000/v1](ref_src/SingleStepTests-m68000/v1).

# Amiga 500 Emulator Architectural & Engineering Guidelines

This repository contains the cycle-exact Amiga 500 emulator written in Rust.
All agentic pair-programming and automated modifications must adhere strictly to the architectural constraints, execution model, and coding guidelines detailed below.

---

## 1. Core Architectural Principles

1. **Target Platforms & Portability**:
   - The emulator core must compile seamlessly for both native desktop targets (x86_64, aarch64) and WebAssembly (`wasm32-unknown-unknown`).
   - The core emulator engine is strictly system-agnostic: zero direct OS or platform dependencies. All host I/O (display rendering, audio playback, disk images, ROM injection) is handled via external buffers and decoupled interfaces.

2. **Clock & Execution Model (Color Clock Phases)**:
   - The primary synchronization unit is the Amiga Color Clock (**CCK**, ~3.54 MHz PAL / ~3.58 MHz NTSC).
   - CPU bus cycles and execution stages are modeled using Color Clock phases: **CCK1** and **CCK2**.
     - $1\ \text{M68000 bus cycle} = 4\ \text{CPU clocks} = 2\ \text{CCK cycles}\ (\text{CCK1} + \text{CCK2})$.
   - All memory bus operations must respect bus readiness via `MemoryBusResult` (`Ready` vs `Blocked`/`Wait`). If Chip RAM is blocked by custom chip DMA, the CPU waits additional CCK cycles without advancing its instruction phase.

3. **Decoupled Architecture & Ownership**:
   - The top-level machine struct (`A500`) owns all major subsystems: `Cpu` (`M68000`), `MemoryBus`, `CycleCounter`, `Agnus`, `Denise`, `Paula`, `CiaA`, and `CiaB`.
   - **No circular references**: Subsystems must not hold direct pointers or circular handles (`Rc<RefCell<...>>`) to each other.
   - All chip coordination, interrupt priority line (IPL 1-6) arbitration, and bus locks are driven in the main machine loop and `MemoryBus`.

4. **Hardware Circuit Simulation & Propagation**:
   - Hardware components simulate real physical circuits: register modifications and bus signals take effect on subsequent clock phases or cycles rather than propagating instantaneously across chips.

5. **Save State Architecture**:
   - Subsystem state structs (`CpuState`, custom chip states) must be decoupled from runtime handles, be fully queryable (read-only snapshots), and implement `serde::Serialize` and `serde::Deserialize`.

---

## 2. Rust Systems & Emulator Coding Guidelines

1. **Guest vs Host Endianness**:
   - The Motorola 68000 is **strictly Big-Endian**, whereas modern host machines are Little-Endian.
   - **Never** perform host-endian pointer casting or `transmute` on guest memory buffers.
   - Always decode and encode multi-byte values using explicit endian conversion helpers:
     ```rust
     let word = u16::from_be_bytes([b0, b1]);
     let long = u32::from_be_bytes([b0, b1, b2, b3]);
     let bytes = val.to_be_bytes();
     ```
   - *Optimization Note (Endianness Bypass)*: Bitwise operations (`AND`, `OR`, `EOR`, `NOT`), zeroing (`CLR`), and memory-to-memory block transfers (DMA, `MOVEM`, `MOVE (An), (Am)`) commute with byte reversal or are byte-order invariant, meaning they can safely bypass endian swapping in performance-critical hot paths.

2. **Zero Host Panics on Guest Code**:
   - Emulated guest code (including malformed binaries, crash dumps, or illegal memory accesses) must **never panic the host process**.
   - Do not use `.unwrap()` or `.expect()` in runtime emulation paths.
   - Unmapped, disconnected, or unreadable address reads must simulate open bus behavior, returning `$FF` for byte reads and `$FFFF` for word reads (standard A500 floating data bus pulled high) rather than indexing out of bounds or panicking. In synthetic test harnesses (such as SingleStepTests flat memory model where unpopulated addresses default to `$00`), this default is explicitly configurable via `MemoryBus::set_unmapped_byte()`.
   - Unaligned word/long accesses must trigger an M68000 Address Error exception (Vector 3).

3. **Arithmetic & Overflow Handling**:
   - In debug builds, Rust panics on integer overflow.
   - In emulation logic, ALU operations and cycle counters must explicitly use wrapping arithmetic:
     ```rust
     let sum = a.wrapping_add(b);
     let diff = a.wrapping_sub(b);
     ```

4. **Zero-Allocation Hot Path**:
   - Hot execution paths (`step()`, `step_cck()`, memory reads/writes, interrupt polling) must perform **zero dynamic heap allocations** (`Vec::new`, `Box::new`, `format!`, `String`).
   - Use fixed arrays, bitflags, or in-place state modifications.

5. **WASM Core Constraints**:
   - In the core crate:
     - No `std::time::Instant::now()` (panics in `wasm32-unknown-unknown` without JS shims).
     - No `std::thread` or OS thread spawning.
     - No `std::fs` calls (load ROMs and disk images as `&[u8]` byte slices passed into public constructors/methods).

6. **Direct Code Flow & Branch-Minimization (Host CPU Pipelining)**:
   - Modern superscalar host CPUs (x86_64, aarch64) feature deep execution pipelines (14–20+ stages) and heavily penalize branch mispredictions (15–20 wasted cycles per stall).
   - Cascaded runtime branches (`match opcode`, `match ea_mode`, `if size == Size::Byte`) in the hot instruction dispatch loop cause severe branch predictor thrashing.
   - For peak performance, the execution core must favor **direct, flattened code flows** (e.g. 65,536-entry direct dispatch table `[fn; 65536]`, direct-threaded handlers, or specialized code generation) where addressing modes, sizes, and registers are statically baked into dedicated handlers, avoiding dynamic runtime conditionals.

7. **Module Cohesion & File Size Guidelines**:
   - **Cohesion over Arbitrary Fragmentation**: Group closely related structs, enums, type definitions, and direct handlers in the same file when they cover the same architectural aspect (e.g. `MemoryBank`, `BankHandler`, and bank functions in `map.rs`). Avoid fragmenting tightly coupled concepts across dozens of micro-files.
   - **Size Thresholds**:
     - *< 300 lines*: Healthy baseline for single-aspect modules and state structures.
     - *300–600 lines*: Ideal sweet spot for cohesive units combining types, enums, and operational logic.
     - *600–800 lines*: Review trigger. Review for multiple responsibilities (SRP violation), independent sub-domains, or test code that should be separated into submodules.
     - *> 800 lines*: Split mandate. Files exceeding 800 lines must be split into submodules unless they meet the criteria for a Recognized Exception.
   - **Recognized Exceptions (Allowed to exceed 800 lines)**:
     - Compile-time static dispatch and lookup tables (e.g. `dispatch_table.rs` with 65,536-entry opcode decoding, BLEP sinc tables).
     - Exhaustive linear instruction decoders or atomic hardware circuit state machines where splitting obscures sequential cycle timing.

8. **Method Inlining Strategy (`#[inline]`, `#[inline(always)]`, `#[inline(never)]`)**:
   - In Rust, `#[inline]` serves two functions: it is an aggressive inlining hint to LLVM, and crucially, it emits intermediate representation (MIR/LLVM IR) into crate metadata, enabling **cross-crate inlining** across workspace crates without requiring whole-program LTO.
   - **Use `#[inline]` on**:
     - Public getters, setters, and single-expression accessors called across crates (e.g., `pub fn chip_ram(&self) -> ChipRamSize`, `pub fn is_chip_ram_blocked(&self) -> bool`).
     - Lightweight forwarding/delegation wrappers (e.g., `pub fn step_cck(&mut self, cck: u64) { self.rtc.step_cck(cck); }`).
     - Endian conversion and byte/word packing helpers.
   - **Use `#[inline(always)]` on**:
     - Ultra-hot arithmetic/logic and CCR condition code flag calculations ($X, N, Z, V, C$) executed multiple times per CCK cycle where function call prologue/epilogue overhead must be strictly eliminated.
   - **Avoid `#[inline]` on**:
     - Medium to large functions (> 15–20 lines of complex control flow) to prevent instruction cache (L1i) bloat and code size explosion.
     - Indirect dispatch table targets (e.g., `BankHandler.read_byte` function pointers, 65,536-entry opcode handlers) that are invoked through pointers and cannot be inlined at the call site.
   - **Use `#[inline(never)]` on**:
     - Cold exception paths, address error dumps, illegal instruction traps, and diagnostic panic paths. Keeping cold recovery logic out-of-line ensures the hot instruction dispatch loop remains dense and contiguous in the host CPU's instruction cache.

9. **Workspace Flat Layout & 3-Tier Re-Export (`pub use`) Strategy**:
   - Keep crate directories in `crates/*` **strictly flat** (no nested crate folders). Express domain containment and API hierarchy through Rust `pub use` re-exports:
     - **Tier 1 (Foundational Blueprint - `config`)**: Machine-wide presets/timings. Never re-exported by peer subsystems.
     - **Tier 2 (Peer Subsystems - `memory_bus`, `m68000`, `agnus`, `denise`, `paula`, `cia`)**: Peers owned by the top-level machine (`A500`). Peers **never re-export other peers**.
     - **Tier 3 (Contained Sub-Components - `rtc`, `copper`, `blitter`)**: Conceptually and physically owned by a specific subsystem. The parent peer **must** re-export them via namespaced modules and convenience shortcuts (`pub use rtc; pub use rtc::RtcMsm6242b;`).
     - **Tier 0 (Top-Level Facade - `a500` machine)**: Owns all peers and acts as the unified gateway for host frontends (`web-wasm`, `desktop-gui`, `cli`).

---

## 3. Knowledge Base & Reference Navigation

- **Design Specifications**: Consult markdown documents under [Obsidian/Amiga/Design](Obsidian/Amiga/Design).
- **Official Hardware Documentation**: Amiga Hardware Reference Manual, 68000 PRMs, and Guru Book reside under [Obsidian/Amiga/Reference](Obsidian/Amiga/Reference) and can be searched via the `rag_search` tool (`amiga-rag`).
- **RAG Tooling & Infrastructure**: The indexing pipeline, CLI indexer (`amiga_rag`), and FastMCP server reside in [`tools/rag`](tools/rag), backed by the local Qdrant vector database (`amiga` collection).
- **Reference Emulator Source Code**: Verified reference implementations (MAME, Moira, Musashi, vAmiga, WinUAE) are located in [ref_src](ref_src).
- **Single-Step Test Vectors**: Official test suite for the M68000 CPU is located in [ref_src/SingleStepTests-m68000/v1](ref_src/SingleStepTests-m68000/v1).

---

## 4. Documentation Maintenance Rule (Definition of Done)

- **Mandatory Final Task:** Whenever an agent (or human developer) implements, refactors, or modifies a subsystem, you **must update the corresponding design document in [Obsidian/Amiga/Design](Obsidian/Amiga/Design) if any architectural decision, timing model, data structure, or hardware quirk has changed or was clarified.**
- **Roadmap Step Completion & Pruning:** Whenever an agent is 100% certain that a roadmap milestone or step in [ROADMAP.md](ROADMAP.md) has been fully implemented and verified (all tests pass 100% green), as part of that **same task/PR you must update [ROADMAP.md](ROADMAP.md)**: remove the detailed completed task from the active implementation list, update the concise completed baseline summary, and renumber/reorder remaining steps so that [ROADMAP.md](ROADMAP.md) always reflects the live, remaining plan.
- **Design Document Pruning & Post-Implementation Cleanup:** Design specifications under [Obsidian/Amiga/Design](Obsidian/Amiga/Design) often contain tentative draft snippets, forward-looking proposals, or hypothetical code sketches written before implementation. Whenever completing a roadmap step or implementing a feature, you **must review and clean up the relevant design documents**: remove obsolete speculative code, prune superseded draft proposals, and ensure the document reflects the finalized, living architectural reality rather than pre-implementation conjectures.
- **Crate Dependency Graph Maintenance:** Whenever crates or workspace dependencies in `Cargo.toml` (new crates, modified inter-crate dependencies, or key external dependencies) are added, altered, or removed, you **must update the Crate Dependency Mermaid Graph in [Obsidian/Amiga/Design/General Architecture.md](Obsidian/Amiga/Design/General%20Architecture.md#2-workspace-crate-architecture--dependencies)**.
- The design documents under `Obsidian/Amiga/Design/` are living, permanent specifications and must always reflect the exact architectural reality of the implementation.

---

## 5. Strict Path Privacy & Workspace Isolation Rule

- **Zero External Paths**: Never write, hardcode, or commit host paths pointing outside the workspace (e.g., personal folders, Google Drive, user directories, absolute disk paths) into any code, configuration, scripts, or documentation inside this repository.
- **Privacy & Portability**: External paths leak private user environment details and break cross-machine portability.
- **Documentation Placeholders**: In documentation, help text, or configuration templates, always use generic placeholders (e.g., `<PATH_TO_VAULT>`, `<PATH_TO_CACHE_DIR>`, `<repo_path>`).
- **User Consultation Required**: If a situation arises where an external path seems needed or requested, you **must stop and ask the user how to solve it** (e.g. via `.env` variables, CLI arguments, or relative paths) rather than assuming, embedding, or exposing external paths.

---

## 6. Specification Compliance & Divergence Escalation Rule (Zero Silent Spec Violations)

- **Living Specification as Ground Truth**: Design specifications under [Obsidian/Amiga/Design](Obsidian/Amiga/Design) and guidelines in `AGENTS.md` define the authoritative architectural truth and hardware behavior for this emulator.
- **Zero Unilateral Divergence**: Agents must **never silently implement code that contradicts or bypasses existing design specifications or rules** (e.g., returning `$00` instead of `$FF` on unmapped memory reads, altering bus contention timings, or silently diverging from hardware models to pass synthetic test vectors).
- **Mandatory Conflict Detection & Escalation**:
  - Whenever an implementation requirement, external test harness expectation (such as SingleStepTests flat memory assumptions), or reference emulator quirk conflicts with the documented specification:
  - You **MUST STOP immediately and present the conflict to the USER before modifying code**.
  - Specifically detail:
    1. What the current design specification / hardware rule requires.
    2. What the conflicting test suite or scenario expects.
    3. The proposed architectural alternatives (e.g. configurable parameters, separate test harnesses vs real emulation modes, or formal spec amendments).
- **Explicit User Decision Required**: No code may deviate from existing documentation without an explicit, recorded decision by the user. Either the documentation is officially updated with user approval, or the code must strictly adhere to the specification.


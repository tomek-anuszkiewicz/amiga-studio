# Amiga 500 Rust Engineering Guidelines & Best Practices

> [!NOTE]
> This document defines the engineering standards, idioms, and best practices for writing Rust code across the Amiga 500 emulator workspace.
> For host CPU hardware mechanical sympathy, see [performance-and-readability.md](file:///d:/Programowanie/Amiga/.agents/rules/performance-and-readability.md).

---

## 1. Core Architectural Philosophies

### 1.1 Explicitness Over Implicit Magic
- **No Clever Obscurity:** Favor explicit, self-documenting code over terse, cryptic shortcuts. In an emulator, readability and traceability to hardware specifications are paramount.
- **Prohibition of Custom Macros (`macro_rules!` Forbidden):** Custom macros obscure call sites, hinder IDE navigation (Go to Definition, Find References), and produce confusing compiler errors. Write explicit functions, direct calls, or `const fn` arrays.
- **Prohibition of Const-Generic Handlers:** Generic functions with constant parameters (e.g. `fn op_foo<const S: usize, const M: usize>(...)`) are forbidden for instruction handlers and execution paths. Code generation and specialized functions are cheap and keep execution paths concrete and debuggable.

### 1.2 Zero Host Panics on Emulated Code
- Emulated guest code must **never crash or panic the host process**.
- Strictly forbid `.unwrap()` and `.expect()` in runtime emulation paths (`step()`, memory accesses, interrupt handling, chip simulation).
- Use defensive defaults or formal error propagation:
  - Unmapped memory reads simulate open bus (`$FF` / `$FFFF`).
  - Unaligned word/long accesses trigger M68000 Vector 3 Address Error exception.
  - Divide-by-zero triggers M68000 Vector 5 Zero Divide trap.

### 1.3 Arithmetic & Overflow Discipline
- In debug builds, Rust panics on integer overflow. In emulator logic, cycle counters and ALU operations must explicitly use wrapping arithmetic (`wrapping_add`, `wrapping_sub`, `wrapping_shl`, `wrapping_shr`).
- Use explicit bitwise masking (`& 0xFF`, `& 0xFFFF`, `& 0xFFFFFF`) when truncating register results to byte, word, or 24-bit address space.

---

## 2. Ownership, Borrowing & Workspace Boundaries

### 2.1 Decoupled Ownership (Zero Circular Handles)
- The top-level machine struct (`A500` or `EmulatorApp`) owns all major subsystems directly:
  - `cpu: Cpu`
  - `bus: MemoryBus`
  - `debugger: Debugger`
  - `agnus: Agnus`, `denise: Denise`, `paula: Paula`, `cia_a: CiaA`, `cia_b: CiaB`
- **Strictly No Circular References:** Never use `Rc<RefCell<...>>` or raw pointers between sibling subsystems.
- If Subsystem A needs to communicate with Subsystem B (e.g. Agnus requesting CPU bus lock), the interaction is arbitrated via `MemoryBus` or top-level machine orchestration.

### 2.2 Workspace Flat Layout & 3-Tier Re-Export Strategy
- All crates reside directly under `crates/*` (strictly flat layout).
- **Tier 1 (Foundational - `config`):** Presets and machine timings. Never re-exported by peer subsystems.
- **Tier 2 (Peer Subsystems - `memory_bus`, `m68000`, `agnus`, etc.):** Peers owned by `A500`. Peers never re-export other peers.
- **Tier 3 (Sub-Components - `rtc`, `copper`, `blitter`):** Re-exported by their parent subsystem (`pub use rtc::*`).
- **Tier 0 (Top-Level Facade - `a500` & `desktop_gui`):** Unified gateway for host frontends.

---

## 3. Memory & Performance Engineering

### 3.1 Big-Endian vs Host Little-Endian Discipline
- The Motorola 68000 is strictly Big-Endian. Modern host architectures (x86_64, aarch64, wasm32) are Little-Endian.
- **Never perform pointer casting or `transmute` on guest memory buffers.**
- Always use explicit byte conversion helpers:
  - `u16::from_be_bytes([b0, b1])`
  - `u32::from_be_bytes([b0, b1, b2, b3])`
  - `val.to_be_bytes()`
- *Endianness Bypass:* Bitwise logic (`AND`, `OR`, `EOR`, `NOT`), zeroing (`CLR`), and bulk memory copies (`MOVE (An), (Am)`, DMA block blits) commute with byte reversal and may operate directly without endian swapping in hot loops.

### 3.2 Zero Dynamic Allocation in Hot Execution Paths
- The hot execution loop (`step()`, `step_cck()`, memory read/write, DMA slot progression) must perform **zero heap allocations** (`Vec::new`, `Box::new`, `String`, `format!`).
- Use fixed-capacity arrays, stack buffers, or pre-allocated ring buffers for trace logs and pipeline latches.

### 3.3 Strategic Inlining
- **`#[inline(always)]`:** Reserved exclusively for ultra-hot ALU calculations and CCR flag formulas ($X, N, Z, V, C$) called multiple times per clock.
- **`#[inline]`:** Applied to small public accessors, single-expression getters/setters called across crate boundaries, and endian conversion helpers so LLVM can optimize across crates without whole-program LTO.
- **`#[inline(never)]`:** Mandatory on cold exception paths (Address Error vector 3 frames, illegal instruction traps, bus error dumps) to keep the primary opcode dispatch loop compact in host L1i cache.
- **No inlining** on large handlers (> 15–20 lines of control flow).

---

## 4. Module Cohesion & File Size Thresholds

To keep the codebase maintainable and prevent monolithic files:
- **< 300 lines:** Baseline for focused modules, state structs, and handlers.
- **300–600 lines:** Sweet spot combining types, enums, and operational logic.
- **600–800 lines:** Review trigger for separable submodules.
- **> 800 lines:** Split mandate into cohesive submodules, unless covered by registered exceptions in `test_architecture_rules.rs` (`LINE_COUNT_EXCEPTIONS`).
- **1:1 Flat Instruction Files:** Every distinct CPU mnemonic resides in its own dedicated flat file directly under `crates/m68000/src/instructions/<mnemonic>.rs` with zero subdirectories.

---

## 5. Testing & Verification Standards

Every implementation or bugfix must satisfy:
1. **Targeted Unit Tests:** Verification of edge cases, register side effects, and boundary conditions.
2. **Automated Architecture Compliance:** Zero-macro, no-unwrap, file size, and inlining rules verified by `cargo test -p test_runner --test test_architecture_rules`.
3. **Format & Lints:** Code formatted with `cargo fmt --all` and validated with `cargo check --all-targets`.

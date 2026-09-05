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

2. **Zero Host Panics on Guest Code**:
   - Emulated guest code (including malformed binaries, crash dumps, or illegal memory accesses) must **never panic the host process**.
   - Do not use `.unwrap()` or `.expect()` in runtime emulation paths.
   - Unmapped or disconnected address reads must simulate open bus behavior (typically returning `$FF` or `$FFFF` on standard A500) rather than indexing out of bounds.
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

---

## 3. Knowledge Base & Reference Navigation

- **Design Specifications**: Consult markdown documents under [Obsidian/Amiga/Design](file:///d:/Programowanie/Amiga/Obsidian/Amiga/Design).
- **Official Hardware Documentation**: Amiga Hardware Reference Manual, 68000 PRMs, and Guru Book reside under [Obsidian/Amiga/Reference](file:///d:/Programowanie/Amiga/Obsidian/Amiga/Reference) and can be searched via the `rag_search` tool (`amiga-rag`).
- **Reference Emulator Source Code**: Verified reference implementations (MAME, Moira, Musashi, vAmiga, WinUAE) are located in [ref_src](file:///d:/Programowanie/Amiga/ref_src).
- **Single-Step Test Vectors**: Official test suite for the M68000 CPU is located in [ref_src/SingleStepTests-m68000/v1](file:///d:/Programowanie/Amiga/ref_src/SingleStepTests-m68000/v1).

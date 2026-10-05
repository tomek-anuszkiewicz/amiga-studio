# Module cohesion examples

Use these examples to identify behavioral boundaries. The [file cohesion rule](../../../rules/file-size-and-cohesion.md) owns limits and exceptions; examples do not require creating every listed file.

## 2. Core Principle: "Aspect-per-File", Not "Class/Struct-per-File"

A foundational architectural mandate across all workspace crates is organizing code by **behavioral aspect / capability**, strictly rejecting the OOP anti-pattern of "one class/struct per file":

1. **Strict Prohibition of Struct-per-File Fragmentation:**
   - Do **NOT** fragment closely related structs, enums, helper types, and algorithms into an explosion of tiny 20-line micro-files (`Breakpoint.rs`, `BreakpointCondition.rs`, `BreakpointHit.rs`, etc.).
   - Fragmenting tightly coupled models across files adds cognitive noise, breaks cross-item visibility, and forces artificial `pub(crate)` ceremony.

2. **Aspect-per-File Cohesion:**
   - A single file must represent a **coherent behavioral aspect or domain capability** of the subsystem, encapsulating all data structures, enums, evaluations, and state mutations required for that capability.
   - **Canonical Reference (`crates/debugger/src/`):**
     - `assembler.rs`: Text-to-binary 68000 instruction parsing, tokenization, and machine code generation.
     - `breakpoints.rs`: Breakpoints, watchpoints, register/memory condition evaluators, and hit-testing logic.
     - `loader.rs`: Binary executable injection into RAM and CPU entrypoint initialization.
     - `session.rs`: Debugger lifecycle, operational state tracking, and session coordination.
     - `stepping.rs`: Instruction flow navigation (step into, step over, step out, run-to-cursor, frame advance).
     - `temporal.rs`: Time-travel rewind ring buffers, state snapshots, and timeline cursor scrubbing.
     - `trace.rs`: Execution trace history ring buffer, disassembly capture, and bottom log display.
   - **Peer Example (`crates/memory_bus/src/`):**
     - `arbitration.rs`: DMA bus contention and priority arbitration.
     - `map.rs`: Address space decoding, memory banks, and open-bus fallback handlers.
     - `registers.rs`: Custom register shadow snapshotting and read/write routing.
     - `bootstrap.rs`: Gary boot overlay toggle mechanics.

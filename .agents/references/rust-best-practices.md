# Rust implementation and review practices

Apply to the selected Rust implementation or review. Specifications own behavioral
requirements. Compiler and Clippy configuration own mechanical settings.

- Support native desktop and `wasm32-unknown-unknown`. Core emulation consumes
  external buffers and performs no host filesystem, thread or wall-clock I/O.
- Keep serializable/queryable snapshots (`serde::Serialize` / `serde::Deserialize`)
  separate from runtime handles. The machine directly owns peer subsystems;
  no circular handles, interior-mutability/shared-threading backchannels or core
  thread spawning. Follow [physical topology](../../Obsidian/Amiga/Design/General%20Architecture.md#11-physical-bus-topology--inter-chip-isolation-invariant).
- Guest errors remain emulated: configurable unmapped reads return `$FF` / `$FFFF`,
  and odd word/long accesses raise Address Error, Vector 3. Runtime code uses no
  `unwrap`, `expect`, `panic!` or `unreachable!`, except the function-scoped
  `clippy::panic` permission on `config::fatal_emulation_error` for internal
  memory-access, CPU bus-timing or mutation-capacity violations. The host terminates
  the failed owner and reports the diagnostic without resuming it. Tests may catch
  that fatal panic to inspect failure, never resume the failed fixture.
- Use explicit wrapping arithmetic for ALU operations and cycle counters; mask
  hardware fields/addresses deliberately.
- Preserve short-circuit evaluation, evaluation order, side effects and hardware
  timing during refactoring. Do not eagerly compute an alternative that the
  original branch would skip.
- Execution, memory-access and interrupt hot paths allocate no heap memory.
- Guest bytes are big-endian. Use explicit endian conversion, never host-endian
  pointer casts or `transmute`. Bitwise operations and raw transfers may bypass
  swaps only when their semantics commute with byte reversal.
- Custom `macro_rules!` and const-generic instruction handlers, decoders or execution
  functions are prohibited. Preserve warning-free builds and configured lints.

For affected crate, export or CPU layout changes read [workspace layout](../../Obsidian/Amiga/Design/General%20Architecture.md#workspace-layout).
Detailed naming, visibility, accessors, inlining and cohesion standards are owned by
[architecture review](architecture-criteria.md); consult them for an
assigned finding or relevant design question, not as an unconditional startup read.

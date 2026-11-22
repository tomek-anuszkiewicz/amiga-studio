# Rust implementation and review practices

Apply these practices to the selected Rust implementation or review. Use the
specifications for behavioral requirements and the compiler and Clippy
configuration for mechanical settings.

- Support native desktop and `wasm32-unknown-unknown`. Core emulation consumes
  external buffers and performs no host filesystem, thread or wall-clock I/O.
- Keep serializable/queryable snapshots (`serde::Serialize` / `serde::Deserialize`)
  separate from runtime handles. The machine directly owns peer subsystems;
  no circular handles, interior-mutability/shared-threading backchannels or core
  thread spawning. Follow [physical topology](../../Obsidian/Amiga/Design/General%20Architecture.md#11-physical-bus-topology--inter-chip-isolation-invariant).
- Emulate guest errors. Configurable unmapped reads return `$FF` / `$FFFF`.
  Word or long accesses at odd addresses raise Address Error, Vector 3. Runtime
  code uses no `unwrap`, `expect`, `panic!` or `unreachable!`, except the function-scoped
  `clippy::panic` permission on `config::fatal_emulation_error` for internal
  memory-access, CPU bus-timing or mutation-capacity violations. The host terminates
  the failed owner and reports the diagnostic without resuming it. Tests may catch
  that fatal panic to inspect failure, never resume the failed fixture.
- Use explicit wrapping arithmetic for ALU operations and cycle counters.
  Deliberately apply masks to hardware fields and addresses.
- Preserve short-circuit evaluation, evaluation order, side effects and hardware
  timing during refactoring. Do not eagerly compute an alternative that the
  original branch would skip.
- Execution, memory-access and interrupt hot paths allocate no heap memory.
- Guest bytes are big-endian. Use explicit endian conversion, never host-endian
  pointer casts or `transmute`. Bitwise operations and raw transfers may bypass
  swaps only when their semantics commute with byte reversal.
- Custom `macro_rules!` and const-generic instruction handlers, decoders or execution
  functions are prohibited. Preserve warning-free builds and configured lints.

For changes that affect crate structure, exports or CPU layout, read the
[workspace layout](../../Obsidian/Amiga/Design/General%20Architecture.md#workspace-layout).
Consult the detailed standards for naming, visibility, accessors, inlining and
cohesion when an assigned finding or design question calls for them. You can
start other work without reading these standards. [Architecture review](architecture-criteria.md)

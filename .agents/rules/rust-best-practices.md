# Rust Engineering Rules

Applies to Rust workspace code; [Rust Guidelines](../../Obsidian/Amiga/Design/Rust%20Guidelines.md) provides design rationale. Compiler and Clippy configuration own mechanical lint settings.

- Guest execution must not panic the host. Runtime emulation uses no `unwrap`, `expect`, `panic!`, or `unreachable!`; tests may explicitly allow panic-oriented assertions.
- Use explicit wrapping arithmetic for ALU operations and cycle counters, and mask hardware fields and addresses deliberately. Use lazy fallbacks when evaluating the alternative would do work.
- Convert guest bytes explicitly as big-endian; never pointer-cast or transmute guest buffers.
- Keep subsystems directly owned by the machine. Do not introduce circular handles, interior-mutability/shared-threading backchannels, or core thread spawning; follow [hardware topology](hardware-bus-topology.md).
- Custom `macro_rules!` and const-generic handlers, decoders, or execution functions are prohibited. Use concrete functions, explicit imports, and readable hardware branches.
- Follow the visibility conventions of neighboring code and expose only what callers need.
- Preserve warning-free compilation. Use [file cohesion](file-size-and-cohesion.md), [workspace boundaries](workspace-structure-and-reexports.md), [performance policy](performance-and-readability.md), and [testing policy](unit-testing-policy.md) rather than duplicating those requirements here.

## Named Numeric Values

- Do not use unnamed numeric literals to encode domain meaning in production logic. Use named constants or existing semantic types for register flags, field masks and shifts, encoded modes, hardware addresses, timing values and domain limits, even when used only once. Comments and hexadecimal or binary notation do not replace semantic names.
- Name the meaning, not the number: prefer `CRB_INPUT_MODE_MASK` over `MASK_03`. Reuse authoritative definitions; place new constants in the nearest owning module and share them only where needed. Equal numeric values with different meanings do not need a shared constant.
- Keep literals whose meaning is clear from the operation, such as zero comparisons, unit increments, ordinary indexing and explicit byte/word width truncation. This exception does not cover a register flag or encoded mode merely because its value is `0` or `1`.
- Test inputs and expected values may retain literal hardware encodings when they independently verify the contract. Do not replace an independent expected encoding with the implementation constant in a way that would hide an incorrect definition.

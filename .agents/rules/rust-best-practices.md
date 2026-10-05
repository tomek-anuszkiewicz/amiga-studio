# Rust Engineering Rules

Applies to Rust workspace code; [Rust Guidelines](../../Obsidian/Amiga/Design/Rust%20Guidelines.md) provides design rationale. Compiler and Clippy configuration own mechanical lint settings.

- Guest execution must not panic the host. Runtime emulation uses no `unwrap`, `expect`, `panic!`, or `unreachable!`; tests may explicitly allow panic-oriented assertions.
- Use explicit wrapping arithmetic for ALU operations and cycle counters, and mask hardware fields and addresses deliberately. Use lazy fallbacks when evaluating the alternative would do work.
- Convert guest bytes explicitly as big-endian; never pointer-cast or transmute guest buffers. Accept borrowed slices and strings instead of concrete container references.
- Keep subsystems directly owned by the machine. Do not introduce circular handles, interior-mutability/shared-threading backchannels, or core thread spawning; follow [hardware topology](hardware-bus-topology.md).
- Custom `macro_rules!` and const-generic handlers, decoders, or execution functions are prohibited. Use concrete functions, explicit imports, and readable hardware branches.
- Use the narrowest visibility required by actual callers. Keep worker modules private or crate-visible; expose only the documented public API. Avoid redundant visibility and test-only zombie runtime methods.
- Getters match field names without `get_`; booleans use natural `is_`, `has_`, or `can_` prefixes. Setters use `set_<field>`. Buffer getters return slices.
- Parameterless `new()` delegates to `Default`; `new` returns `Self`. Prefer derivable implementations, derive `Clone` for `Copy` types, and provide `Debug` on public structs/enums.
- Preserve warning-free compilation. Use [file cohesion](file-size-and-cohesion.md), [workspace boundaries](workspace-structure-and-reexports.md), [performance policy](performance-and-readability.md), and [testing policy](unit-testing-policy.md) rather than duplicating those requirements here.

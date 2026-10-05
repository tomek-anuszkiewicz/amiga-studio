# Rust File Size and Cohesion

Applies to production `.rs` files under `crates/*/src/`; documentation and reference manuals have no line-count limit.

- Organize each file around a behavioral aspect or capability, keeping related types and algorithms together. Avoid one-struct-per-file fragmentation.
- Below 600 lines is the usual cohesive range; 600-800 lines triggers responsibility review. Above 800 lines requires decomposition unless covered by a registered exception.
- Split independent responsibilities even below the limit. Separate serializable state from heavy simulation where coherent; keep tests outside `src/` per [unit-testing-policy.md](unit-testing-policy.md).
- Recognized exceptions include static dispatch/LUT tables and indivisible linear instruction decoders or hardware state machines. Never add `LINE_COUNT_EXCEPTIONS` merely to silence a failure: decompose or present metrics and request explicit approval. Remove stale exceptions when a file disappears or reaches 800 lines or fewer.
- CPU instructions remain flat in `crates/cpu/src/instructions/`, one mnemonic per file; no subdirectories or umbrella files combining distinct mnemonics. Oversized instruction exceptions remain flat.
- Existing grouping exceptions are `move_sr_ccr.rs`, `logic_sr_ccr.rs`, size-based `move_b.rs` / `move_w.rs` / `move_l.rs`, and condition families `bcc.rs` / `dbcc.rs` / `scc.rs`.

Use [refactor-split-module](../skills/refactor-split-module/SKILL.md) for decomposition and its supporting examples. Preserve [workspace boundaries](workspace-structure-and-reexports.md) and [clean refactoring](clean-break-refactoring.md).

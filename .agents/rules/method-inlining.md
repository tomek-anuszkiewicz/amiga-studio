---
trigger: model_decision
description: Inlining strategy (#[inline], #[inline(always)], #[inline(never)]) for performance-critical and cross-crate Rust methods.
---

# Method Inlining Strategy (`#[inline]`, `#[inline(always)]`, `#[inline(never)]`)

## 1. Core Mechanics in Rust
- `#[inline]` is a strong hint to LLVM, but its primary role in a multi-crate workspace is enabling **cross-crate inlining** (exporting MIR/LLVM IR into crate metadata without requiring whole-program LTO).
- `#[inline(always)]` mandates inlining at the LLVM level to eliminate call/return prologue and epilogue overhead.
- `#[inline(never)]` forces the compiler to keep the function out-of-line, ensuring cold error recovery does not inflate hot execution paths.

## 2. Inlining Decision Matrix

| Attribute | When to Use | Examples in this Codebase |
| :--- | :--- | :--- |
| **`#[inline]`** | • Public getters, setters, accessors called across crates<br/>• Lightweight forwarding/delegation wrappers<br/>• Endian and byte packing helpers | `pub fn chip_ram(&self) -> ChipRamSize`<br/>`pub fn is_chip_ram_blocked(&self) -> bool`<br/>`pub fn step_cck(&mut self, cck: u64) { self.rtc.step_cck(cck); }` |
| **`#[inline(always)]`** | • Ultra-hot ALU condition code calculations ($X, N, Z, V, C$)<br/>• Operations executed multiple times per CCK cycle | CCR flag evaluations for byte/word/long math<br/>`self.update_ccr_add(...)` |
| **No attribute** (Default) | • Functions with > 15–20 lines of control flow<br/>• Indirect table targets (e.g. `BankHandler.read_byte`, opcode tables) | `fn read_chip_ram(...)`<br/>`fn execute_move_w(...)` |
| **`#[inline(never)]`** | • Cold exception paths and traps<br/>• Panic / unreachable / diagnostic dumps | Vector 3 Address Error stack frame creation<br/>Illegal instruction reporter |

## 3. Anti-Patterns to Avoid
1. **Do not put `#[inline]` on large functions**: It bloats the compiled binary, degrades compiler optimization heuristics, and increases code size needlessly.
2. **Do not put `#[inline]` on function-pointer targets**: If a function is called indirectly through `[BankHandler; 256]` or `[OpcodeHandler; 65536]`, the compiler cannot inline it through the pointer at the call site anyway.
3. **Do not omit `#[inline]` on public accessors across workspace crates**: Without `#[inline]`, callers in downstream crates cannot inline small accessor methods unless costly cross-crate LTO is enabled in `Cargo.toml`.


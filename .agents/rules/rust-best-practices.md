---
trigger: model_decision
description: Rust systems programming best practices, safe borrowing, zero unwraps, wrapping math, and unit tests.
---

# Rust Engineering Best Practices & Systems Guidelines

All Rust code across the Amiga 500 emulator workspace must strictly adhere to these engineering guidelines.

---

## 1. Safety & Error Discipline
- **Zero Host Panics on Guest Code:** Runtime emulation code (`step()`, memory accesses, interrupt handling, chip registers) must **never** call `.unwrap()` or `.expect()`. Handle open bus, unaligned access, or invalid opcodes defensively.
- **Wrapping Arithmetic:** In emulator ALU and cycle counting, always use explicit wrapping arithmetic (`wrapping_add`, `wrapping_sub`, `wrapping_shl`, `wrapping_shr`) to avoid debug overflow panics.
- **Explicit Bit Masking:** Explicitly mask results (`& 0xFF`, `& 0xFFFF`, `& 0xFFFFFF`) when truncating registers or memory addresses.

---

## 2. Explicitness & Code Clarity
- **Strict Prohibition of User-Defined Macros (`macro_rules!` Forbidden):** Custom macros are forbidden across the codebase. Write explicit, self-documenting Rust functions, direct calls, or compile-time `const fn` arrays.
- **Prohibition of Const-Generic Functions with Constant Parameters:** Const generics (`<const N: usize>`) are forbidden for instruction handlers, decoding logic, and execution paths. Write concrete, specialized functions.
- **No Clever Obscurity:** Prioritize readability and direct 1:1 hardware traceability over cryptic micro-optimizations that LLVM already handles.

---

## 3. Ownership & Memory Hierarchy
- **Zero Circular Handles:** Never use `Rc<RefCell<...>>` or raw pointers between sibling subsystems. All subsystems are owned directly by the top-level machine (`A500` or `EmulatorApp`).
- **Big-Endian Guest vs Little-Endian Host:** Never perform pointer casts or `transmute` on guest memory buffers. Always use explicit byte conversion helpers (`u16::from_be_bytes`, `u32::from_be_bytes`).
- **Zero Allocations in Hot Paths:** Hot execution paths must perform zero dynamic heap allocations (`Vec`, `Box`, `String`, `format!`). Use fixed-capacity arrays or in-place state.

---

## 4. Module Cohesion & File Sizing
- Keep Rust source files under **800 lines** in `crates/*/src/` (unless covered by registered exceptions in `LINE_COUNT_EXCEPTIONS`).
- Maintain a flat instruction hierarchy directly under `crates/cpu/src/instructions/<mnemonic>.rs` with zero subdirectories.
- Workspace layout under `crates/*` must remain strictly flat (3-tier re-export strategy).

---

## 5. Principle of Minimum Visibility (Least Privilege Visibility)
- **Minimum Required Visibility:** Functions, structs, methods, constants, and modules must strictly use the narrowest visibility under which they currently function.
- **Private by Default:** All items are private (`fn`, `struct`, `const`) unless access across files is genuinely required.
- **`pub(crate)` for Internal Collaboration:** Use `pub(crate)` when an item must be shared between modules within the same crate. Never default to `pub` for internal helpers, execution engines, or dispatch callbacks.
- **`pub` Strictly for External Public API:** Elevate to `pub` only when an item forms part of the crate's documented public surface consumed by downstream peer crates or host frontends (`machine_loop`, `gui`).
- **Encapsulate Internal Modules:** Crates must never expose internal worker submodules (such as `instructions`, `decoders`, internal callbacks) via `pub mod`. Use `pub(crate) mod` to maintain an uncluttered crate API.
- **Eliminate Visibility Leaks for Dead Code Detection:** Leaking `pub` visibility prevents the Rust compiler and static analysis tools from identifying unused dead code. Restricting visibility ensures dead or zombie code is surfaced immediately.

---

## 6. Comprehensive Unit Test Coverage
- **Mandatory Unit Tests for Testable Logic:** Every newly created or modified Rust source file containing testable domain logic, algorithmic transformations, state machines, hardware models, statistical calculations, builders, or parsers must have corresponding unit tests.
- **Placement & Structure:** Unit tests must be placed strictly in dedicated test files under `crates/<crate>/tests/test_<name>.rs` per `unit-testing-policy.md` (zero inline tests in `src/`).
- **Pragmatic Scope:** Pure struct declarations or thin forwarders without branching or business logic may rely on parent integration tests. However, any module implementing algorithms, parsing, state mutations, filtering, statistics, or hardware circuits must have dedicated unit tests verifying happy paths, boundary conditions, zero/empty states, and failure modes.

---

## 7. Authoritative Rust Design Specifications & Delegation

When designing Rust data models, trait interfaces, and systems boundaries, agents must adhere to:
- [`Rust Guidelines.md`](../../Obsidian/Amiga/Design/Rust%20Guidelines.md): Project-wide Rust systems idioms, memory layout patterns, and error handling architecture.



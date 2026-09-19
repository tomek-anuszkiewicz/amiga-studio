---
trigger: model_decision
description: Rust systems programming best practices, safe borrowing, zero unwraps, wrapping math, and unit tests.
---

# Rust Engineering Best Practices & Systems Guidelines

All Rust code across the Amiga 500 emulator workspace must strictly adhere to these engineering guidelines.

---

## 1. Safety & Error Discipline
- **Zero Host Panics on Guest Code:** Runtime emulation code (`step()`, memory accesses, interrupt handling, chip registers) must **never** call `.unwrap()`, `.expect()`, `panic!()`, or `unreachable!()`. Handle open bus, unaligned access, or invalid opcodes defensively. Mechanically enforced in production code (`crates/*/src/`) via compiler lints `clippy::unwrap_used = "deny"`, `clippy::expect_used = "deny"`, `clippy::panic = "deny"`, and `clippy::unreachable = "deny"`. Integration test suites (`crates/*/tests/*.rs`) are exempt via `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`.
- **Wrapping Arithmetic:** In emulator ALU and cycle counting, always use explicit wrapping arithmetic (`wrapping_add`, `wrapping_sub`, `wrapping_shl`, `wrapping_shr`) to avoid debug overflow panics.
- **Explicit Bit Masking & Natural Hardware Casting:** Explicitly mask results (`& 0xFF`, `& 0xFFFF`, `& 0xFFFFFF`) when isolating register fields or memory bus addresses. However, generic linter truncation warnings (`clippy::cast_possible_truncation = "allow"`) are intentionally allowed project-wide to avoid polluting physical hardware registers, ALU routines, and M68000 ISA implementations with hundreds of redundant defensive casts.
- **Non-Eager Fallback Closures:** Never evaluate function calls eagerly inside fallback options (`unwrap_or(func())`). Use lazy closure evaluation (`unwrap_or_else(|| func())`) to preserve short-circuiting. Mechanically enforced via `clippy::or_fun_call = "deny"`.
- **Boolean Logic Clarity & De-Morgan Simplification:** Keep boolean conditions minimal, declarative, and free of redundant terms (e.g. `(n == v) && !z` for M68000 `GT`). Mechanically enforced via `clippy::nonminimal_bool = "deny"` and `clippy::needless_bool = "deny"`.

---

## 2. Explicitness & Code Clarity
- **Strict Prohibition of User-Defined Macros (`macro_rules!` Forbidden):** Custom macros are forbidden across the codebase. Write explicit, self-documenting Rust functions, direct calls, or compile-time `const fn` arrays. Enforced via `test_architecture_rules.rs`.
- **Prohibition of Const-Generic Functions with Constant Parameters:** Const generics (`<const N: usize>`) are forbidden for instruction handlers, decoding logic, and execution paths. Write concrete, specialized functions. Enforced via `test_architecture_rules.rs`.
- **Explicit Imports (Zero Wildcard Imports):** Wildcard imports (`use module::*;`) obscure symbol origin, pollute namespaces, and break IDE navigation. Explicitly enumerate all imported items (`use module::{ItemA, ItemB};`). Mechanically enforced via `clippy::wildcard_imports = "deny"`.
- **Hardware Architecture Preservation (Disabled Linter Collapsing):** In an emulator, distinct opcode bit patterns or register addresses legitimately share execution logic, and multi-stage hardware timing checks (CCK phases, DMA arbitration) must remain transparently sequential. Clippy's `match_same_arms = "allow"`, `collapsible_if = "allow"`, and `collapsible_else_if = "allow"` are intentionally disabled to prevent linters from destroying 1:1 hardware readability into collapsed condition soup.
- **No Clever Obscurity:** Prioritize readability and direct 1:1 hardware traceability over cryptic micro-optimizations that LLVM already handles.

---

## 3. Ownership & Memory Hierarchy
- **Zero Circular Handles & Multi-Threading Primitives:** Never use `Rc`, `RefCell`, `Arc`, `Mutex`, `RwLock`, `mpsc::Sender`, or `mpsc::Receiver` between subsystems or in machine state. Multi-threading primitives and thread spawning (`std::thread::spawn`) are strictly forbidden in core machine logic. All subsystems are owned directly by the top-level machine (`A500` or `EmulatorApp`). Mechanically enforced via `clippy::disallowed_types` and `clippy::disallowed_methods`.
- **Big-Endian Guest vs Little-Endian Host:** Never perform pointer casts or `transmute` on guest memory buffers. Always use explicit byte conversion helpers (`u16::from_be_bytes`, `u32::from_be_bytes`). Enforced via `clippy::cast_ptr_alignment = "deny"` and `clippy::transmute_ptr_to_ptr = "deny"`.
- **Zero Allocations in Hot Paths:** Hot execution paths must perform zero dynamic heap allocations (`Vec`, `Box`, `String`, `format!`). Use fixed-capacity arrays or in-place state. Enforced via `clippy::vec_box = "deny"` and `clippy::box_collection = "deny"`.
- **Borrow Views over Containers:** Functions inspecting buffers or sequences must accept borrowed slices (`&[T]`, `&mut [T]`) rather than concrete heap containers (`&Vec<T>`, `&mut Vec<T>`). Accept `&str` instead of `&String`. Enforced at compiler/AST level via `clippy::ptr_arg = "deny"`.

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
- **Redundant Visibility Modifiers:** Never add `pub(crate)` qualifiers to items or modules inside an already-private parent module (where `pub` already achieves crate-private visibility). Enforced via `clippy::redundant_pub_crate = "deny"`.
- **Eliminate Visibility Leaks for Dead Code Detection:** Leaking `pub` visibility prevents the Rust compiler and static analysis tools from identifying unused dead code. Restricting visibility ensures dead or zombie code is surfaced immediately.

---

## 6. Struct Encapsulation & Accessor Rule

### Category A: Value Objects / POD Structs
Matches pure data structs (primitives, raw numbers, small `Copy` types) with public fields or trivial side-effect-free setters.

**Rules:**
- Make all fields private.
- Add `pub const fn new(...) -> Self`.
- Add `#[inline(always)] pub const fn <field>(&self)` getters (return `T` if `Copy`, else `&T`).
- Add `#[inline(always)] pub const fn set_<field>(&mut self, val: T)` setters.
- Derive `Debug, Clone, Copy, PartialEq, Eq` when possible.

**Example:**
```rust
// BEFORE:
pub struct StereoSample { pub left: i16, pub right: i16 }

// AFTER:
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StereoSample {
    left: i16,
    right: i16,
}

impl StereoSample {
    #[inline(always)]
    pub const fn new(left: i16, right: i16) -> Self { Self { left, right } }

    #[inline(always)]
    pub const fn left(&self) -> i16 { self.left }

    #[inline(always)]
    pub const fn right(&self) -> i16 { self.right }

    #[inline(always)]
    pub const fn set_left(&mut self, left: i16) { self.left = left; }

    #[inline(always)]
    pub const fn set_right(&mut self, right: i16) { self.right = right; }
}
```

### Category B: All Remaining / Complex Structs
Applies to structs containing allocations, handles, non-primitive state, or business logic invariants.

**Rules:**
- All fields must remain strictly private.
- Provide appropriate constructors (`new` or `try_new` if fallible/validated).
- Accessors:
  - Use `pub const fn <field>(&self) -> &T` if evaluatable at compile time.
  - Fall back to `pub fn <field>(&self) -> &T` otherwise.
- Setters:
  - Only provide if explicitly required by domain logic.
  - Must enforce necessary invariants and validations.

**Example:**
```rust
pub struct AudioBuffer {
    channels: usize,
    data: Vec<i16>,
}

impl AudioBuffer {
    pub fn try_new(channels: usize, capacity: usize) -> Result<Self, &'static str> {
        if channels == 0 {
            return Err("channels must be greater than zero");
        }
        Ok(Self {
            channels,
            data: Vec::with_capacity(capacity),
        })
    }

    #[inline]
    pub const fn channels(&self) -> usize {
        self.channels
    }

    #[inline]
    pub fn data(&self) -> &[i16] {
        &self.data
    }
}
```

### Method Naming & Accessor Conventions

1. **Standard Getters:**
   - Must exactly match the field name (do NOT use a `get_` prefix).
   - Pattern: `pub const fn <field>(&self) -> T` (or `&T` if non-Copy).
   - Example: Field `sample_rate: u32` -> getter `pub const fn sample_rate(&self) -> u32`.

2. **Boolean Getters:**
   - Must start with the `is_` prefix (or retain natural boolean prefixes like `has_`, `can_` if already present in the field name).
   - If the field is named `enabled: bool`, the getter is `pub const fn is_enabled(&self) -> bool`.
   - If the field already has an `is_` prefix (e.g. `is_active: bool`), do not duplicate it (`pub const fn is_active(&self) -> bool`).

3. **Setters:**
   - Must start with the `set_` prefix followed by the field name.
   - Pattern: `pub const fn set_<field>(&mut self, value: T)`.
   - Example: Field `volume: u8` -> setter `pub const fn set_volume(&mut self, volume: u8)`.
   - Example (Boolean): Field `enabled: bool` -> setter `pub const fn set_enabled(&mut self, enabled: bool)`.

Example:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelConfig {
    volume: u8,
    enabled: bool,
}

impl ChannelConfig {
    #[inline(always)]
    pub const fn new(volume: u8, enabled: bool) -> Self {
        Self { volume, enabled }
    }

    // Standard getter (matches field name)
    #[inline(always)]
    pub const fn volume(&self) -> u8 {
        self.volume
    }

    // Boolean getter (starts with is_)
    #[inline(always)]
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    // Setters (start with set_)
    #[inline(always)]
    pub const fn set_volume(&mut self, volume: u8) {
        self.volume = volume;
    }

    #[inline(always)]
    pub const fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}
```

4. **Collection Getters (Slice Views):**
   - Getters exposing internal buffers or sequences must return borrowed slices (`&[T]` or `&mut [T]`), never references to concrete containers (`&Vec<T>`).
   - Example: Field `data: Vec<i16>` -> getter `pub fn data(&self) -> &[i16]`.

---

## 7. Trait Derives, Constructors & Constructor Discipline
- **Default for Parameterless Constructors:** Always implement or derive `Default` if a parameterless constructor (`new()`) exists, ensuring `new()` delegates to `Self::default()`. Enforced at compiler/AST level via `clippy::new_without_default = "deny"`.
- **Constructors Returning Self:** Any method named `new` must return `Self`. Enforced via `clippy::new_ret_no_self = "deny"`.
- **Derive Clone on Copy:** Never manually implement `Clone` when the type implements `Copy`. Enforced via `clippy::expl_impl_clone_on_copy = "deny"`.
- **Derivable Implementations:** Prefer `#[derive(Default)]` over manual implementations when all fields implement `Default`. Enforced via `clippy::derivable_impls = "deny"`.
- **Mandatory Debug Trait:** All public enums and structs must derive `Debug`. Enforced at compiler level via `missing_debug_implementations = "deny"`.

---

## 8. Comprehensive Unit Test Coverage
- **Mandatory Unit Tests for Testable Logic:** Every newly created or modified Rust source file containing testable domain logic, algorithmic transformations, state machines, hardware models, statistical calculations, builders, or parsers must have corresponding unit tests.
- **Placement & Structure:** Unit tests must be placed strictly in dedicated test files under `crates/<crate>/tests/test_<name>.rs` per `unit-testing-policy.md` (zero inline tests in `src/`).
- **Pragmatic Scope:** Pure struct declarations or thin forwarders without branching or business logic may rely on parent integration tests. However, any module implementing algorithms, parsing, state mutations, filtering, statistics, or hardware circuits must have dedicated unit tests verifying happy paths, boundary conditions, zero/empty states, and failure modes.

---

## 9. Authoritative Rust Design Specifications & Delegation

When designing Rust data models, trait interfaces, and systems boundaries, agents must adhere to:
- [`Rust Guidelines.md`](../../Obsidian/Amiga/Design/Rust%20Guidelines.md): Project-wide Rust systems idioms, memory layout patterns, and error handling architecture.




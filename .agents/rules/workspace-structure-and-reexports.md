---
trigger: model_decision
description: Workspace flat crate layout in crates/* and 3-tier re-export (pub use) hierarchy.
---

# Workspace Flat Layout & 3-Tier Re-Export (`pub use`) Strategy

## 1. Core Principle: Flat on Disk, Hierarchical in Code
In this repository, all crate directories in `crates/*` remain **strictly flat** (e.g. `crates/config`, `crates/rtc`, `crates/memory_bus`, `crates/m68000`).
Architectural ownership, containment, and subsystem boundaries are expressed **in Rust code via `pub use` re-exports**, never through deeply nested directories.

---

## 2. The 3-Tier Architecture & Re-Export Taxonomy

```
Tier 0: Top-Level Facade (a500 / amiga_core)
  │     Re-exports all peers for host frontends (GUI, WASM, CLI)
  ▼
Tier 1: Foundational Blueprint (config)
        System-wide immutable presets; NEVER re-exported by peers
  ▼
Tier 2: Peer Subsystems (memory_bus, m68000, agnus, denise, paula, cia)
        Orchestrated side-by-side by A500; NEVER re-export each other
  ▼
Tier 3: Contained Sub-Components (rtc, copper, blitter, audio_dacs)
        Belong to a specific subsystem; MUST be re-exported by their parent peer
```

### Tier 1: Foundational Blueprint (`crates/config`)
- Holds machine-wide specifications (`A500Config`, `VideoStandard`, `A500Preset`, RAM sizes).
- Used by all subsystems (MemoryBus, Agnus, Denise, Paula, CIAs).
- **Rule**: Peer subsystems do **not** re-export `config` as if they own it. Downstream consumers configure subsystems by importing `config` directly or via the Tier 0 facade.

### Tier 2: Peer Subsystems (`memory_bus`, `m68000`, `agnus`, `denise`, `paula`, `cia`)
- All owned as parallel peers by the top-level machine struct (`A500`).
- **Rule**: Peers **never re-export other peers**. 
  - `m68000` does not re-export `memory_bus`.
  - `memory_bus` does not re-export `agnus` or `cpu`.
  - Any bus coordination is handled via transient contexts (e.g., `BusContext`) in the top-level machine loop.

### Tier 3: Contained Sub-Components (`rtc`, `copper`, `blitter`, `audio`)
- Dedicated crates that physically or logically belong to a single parent subsystem:
  - `rtc` belongs to `memory_bus` (passive peripheral at `$DC0000`).
  - `copper` and `blitter` belong to `agnus` (Agnus coprocessors).
  - `audio_channel` belongs to `paula`.
- **Rule**: The parent peer **must** re-export the child crate:
  ```rust
  // In crates/memory_bus/src/memory_bus.rs
  pub use rtc;                   // Module namespace: memory_bus::rtc::*
  pub use rtc::RtcMsm6242b;      // Flat convenience shortcut
  ```

### Tier 0: Top-Level Facade (`crates/a500` / `crates/amiga`)
- The machine chassis that owns all peers.
- **Rule**: Acts as the unified gateway for host frontends (Desktop GUI, WebAssembly, CLI):
  ```rust
  // In crates/a500/src/a500.rs (future)
  pub use config;
  pub use memory_bus;
  pub use m68000;
  pub use agnus;
  pub use denise;
  pub use paula;
  pub use cia;
  ```

---

## 3. Re-Export Syntax Conventions

| Goal | Syntax | When to Use |
| :--- | :--- | :--- |
| **Namespaced Module** | `pub use child_crate;` | When the child crate contains multiple types, registers, states, or enums (e.g. `pub use rtc;`). |
| **Primary Type Shortcut** | `pub use child_crate::MainStruct;` | For the 1–2 most prominent structs to prevent verbose typing (e.g. `pub use rtc::RtcMsm6242b;`). |
| **Avoid Wildcard Roots** | ❌ `pub use child_crate::*;` | Do not glob-reexport child crates at the root to prevent naming collisions (e.g. two crates defining `State`). |

---

## 4. Anti-Patterns to Avoid
1. **The "God Bus" Absorption**: Do not re-export active coprocessors (`agnus`, `paula`, `cia`) from `memory_bus`. They are autonomous peers, not child storage buffers.
2. **Peer Leaks**: Do not re-export a peer subsystem just because a method accepts it. Use dependency inversion or trait interfaces instead.
3. **Deep Directory Nesting**: Do not place crates inside other crate folders (e.g. `crates/memory_bus/rtc`). Keep all crates flat in `crates/*` and let `pub use` establish the namespace.
4. **Compatibility Shims & Stale Aliases**: Never introduce transitional dummy wrapper modules (`pub mod former { pub use new::*; }`) or import aliases (`use new as old;`) to delay updating callers.

---

## 5. Prohibition of Backward-Compatibility Shims & Stale Aliases (Mandatory Atomic Refactoring)

In this closed repository with zero external downstream semver consumers, all refactorings are **monolithic and atomic**:
1. **Zero Backward-Compatibility Shims:**
   - Never introduce artificial dummy inline wrapper modules (e.g. `pub mod former_mod { pub use new_crate::*; }`) to emulate obsolete crate or file structures.
   - Never leave transitional import aliases (e.g. `use new_fn as old_fn;`) across files or test harnesses.
   - Never write forwarding stub functions or deprecated wrappers when renaming or relocating functionality.
2. **Atomic Workspace Updates:**
   - Whenever extracting a submodule into a standalone crate or relocating types, immediately search and update all call sites across the entire repository (`crates/*`, `tests/*`) in the exact same commit.
   - Do not defer call-site updates behind compatibility shims.
3. **No Legacy Compatibility Comments:**
   - Do not annotate active convenience methods or public APIs with "legacy compatibility" doc comments. If a method is obsolete, remove it; if it is active, describe its actual functional behavior.

---

## 6. Named Crate Roots & Zero Generic `lib.rs` Mandate

To improve searchability, eliminate ambiguous file tabs in editors, and guarantee consistent 1:1 crate-to-root alignment:
1. **Named Entry Points**:
   - Every library crate under `crates/<crate_name>/` must name its root entry point file `src/<crate_name>.rs` matching the crate directory name (e.g. `crates/agnus/src/agnus.rs`, `crates/m68000/src/m68000.rs`).
   - The crate's `Cargo.toml` must explicitly configure the library target path:
     ```toml
     [lib]
     path = "src/<crate_name>.rs"
     ```
2. **Strict Prohibition of Generic `lib.rs`**:
   - Files named `lib.rs` are **strictly forbidden** anywhere across workspace crates (`crates/`) and helper tools (`tools/`).
   - Enforced by automated architecture test `test_named_crate_roots_and_zero_generic_lib_rs`.


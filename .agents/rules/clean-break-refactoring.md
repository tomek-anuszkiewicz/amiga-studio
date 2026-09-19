# Clean-Break Refactoring & Zero Unsolicited Backward Compatibility

This rule governs all code refactoring, API renaming, structural modifications, and architectural redesigns across the entire repository.

---

## 1. The Core Mandate: Complete, Workspace-Wide Cutover

Whenever instructed to rename, refactor, replace, or redesign a method, struct, field, enum variant, or subsystem:
- **Execute a clean, complete break across the entire workspace.**
- Update every single caller, test fixture, test assertion, and documentation reference in the same change.
- **Zero legacy residue**: Never leave the old mechanism running in parallel with the new mechanism. A change is a change everywhere.

---

## 2. Strictly Prohibited Backward-Compatibility Shims

Unless the user **explicitly** requests backward compatibility in their prompt (e.g. *"keep legacy aliases for X"*), the following patterns are strictly forbidden:

### A. Zero Legacy Type Aliases
- **Forbidden:** Leaving compatibility type aliases like `pub type OldName = NewName;` (e.g. `pub type MemoryBus = PhysicalMemory;`).
- **Required:** Rename the type and update all `use` statements, variable declarations, and type signatures across all workspace crates and test suites.

### B. Zero Forwarding / Deprecation Wrappers
- **Forbidden:** Keeping old functions or methods that forward calls to new implementations (e.g. `pub fn old_method(&mut self) { self.new_method(); }`).
- **Required:** Delete the old method immediately and update all call sites to call the new method directly.

### C. Zero Dual-Path Fallback Branches
- **Forbidden:** Adding runtime branching or special-case conditions to support both the old and new behaviors simultaneously (e.g. `if !new_thing_present { fallback_to_legacy_path(); }`).
- **Required:** Commit 100% to the new design. Migrate all callers and tests to the new invariant.

### D. Zero Compatibility Re-Exports & Dual Namespaces
- **Forbidden:** Preserving obsolete re-exports or duplicate module paths to keep unmigrated files compiling.
- **Required:** Fix the imports in the unmigrated files. Every file must use the canonical new path.

### E. Zero `#[deprecated]` Annotations
- **Forbidden:** Slapping `#[deprecated]` on dead or obsolete code to avoid updating callers.
- **Required:** In a closed monorepo, deprecated code is pure technical debt. Delete obsolete code completely. Mechanically enforced via `deprecated = "deny"` in root `Cargo.toml [workspace.lints.rust]`, causing any use of deprecated items to immediately fail the build.

---

## 3. The Closed-World Monorepo Invariant

AI coding models often default to backward-compatibility shims because their training data is saturated with open-source public libraries (crates.io, npm, PyPI), where breaking changes break downstream users.

**That context does NOT apply here:**
1. **Zero External Consumers:** This repository is an internal, closed-world application, not a public library. No external consumers exist.
2. **Atomic Control:** We control every line of code, every test, and every configuration file simultaneously.
3. **Purity & Cohesion:** Unsolicited backward-compatibility shims create "Frankenstein architectures" where multiple conflicting abstractions coexist, obscuring hardware silicon behavior and confusing developers.

---

## 4. Strict Opt-In Protocol

- **Default Stance:** Total, uncompromising cutover. Clean break.
- **The Only Exception:** Backward compatibility is granted **ONLY** when the user explicitly commands it in their prompt. If the user didn't ask for it, do not provide it.

---

## 5. Refactoring Verification Checklist
Before completing any refactoring or renaming task, verify:
- [ ] Are all old methods, structs, and aliases completely deleted (zero legacy shims)?
- [ ] Was `grep_search` executed across the entire repository to confirm zero remaining references to old symbols?
- [ ] Are all test files and fixtures updated to the new API?
- [ ] Are all dual-path fallback branches eliminated?

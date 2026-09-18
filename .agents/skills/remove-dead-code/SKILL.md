---
name: remove-dead-code
description: Systematically detect, audit, and eliminate completely dead code and test-only zombie code (symbols tested in unit tests but unused in production).
---

# Recipe: Dead Code & Test-Only Zombie Elimination (`remove-dead-code`)

This skill provides a systematic procedure for identifying, analyzing, and purging dead code across the Rust workspace, with specialized detection for **Test-Only Zombie Code** (symbols that appear alive only because unit tests exercise them, but are completely unreferenced by production emulation code).

---

## 1. The Two Classes of Dead Code

1. **Class 1: Completely Dead Symbols (💀)**
   - Methods, constants, or types with **zero callers anywhere** in `crates/*/src/` or `crates/*/tests/`.
   - Action: Safe to delete immediately.

2. **Class 2: Test-Only Zombie Code (🧟)**
   - Symbols with **zero callers in production code (`crates/*/src/`)**, but referenced in unit/integration tests (`crates/*/tests/`).
   - Why this happens: A helper method was written, a unit test was written to test it, and later refactorings bypassed or superseded the method in the machine loop/bus. The test kept passing, creating the illusion that the code was active.
   - Action: Audit whether the method is an intended host-facing peripheral API (e.g. `keyboard::key_down` used by GUI/SDL event loops) or vestigial scaffolding. If vestigial, purge both the method and its orphaned unit test.

3. **Class 3: Crate-Internal Unused Symbols (The "Visibility Downgrade" Trick)**
   - In library crates (`[lib]`), `rustc` treats every `pub` item as an exported API and silences dead code warnings.
   - Temporarily switching `pub` to `pub(crate)` on suspect modules unmasks `rustc`'s internal reachability analysis via `cargo check`.

---

## 2. Step-by-Step Pruning Workflow

### Step 1: Automated Static Audit
Run the workspace dead & zombie code auditor:
```powershell
python tools/harness/detect_dead_code.py
```
To inspect a single crate or filter by category:
```powershell
python tools/harness/detect_dead_code.py --crate <crate_name>
python tools/harness/detect_dead_code.py --dead-only
python tools/harness/detect_dead_code.py --zombies-only
```

### Step 2: Triage Zombie Symbols
For each symbol reported under `[TEST-ONLY ZOMBIES]`:
1. Check if it represents an **external Host I/O boundary**:
   - Host input injection (e.g. `keyboard::key_down`, `game_ports::plug_port1`, `floppy::insert_disk`). These are intended for the frontend GUI / CLI runner.
2. If it does NOT represent an external host interface:
   - Verify why production code no longer calls it.
   - Mark both the production declaration and its test for clean-break removal.

### Step 3: Safe Removal
1. Delete the dead symbol in `crates/<crate>/src/`.
2. Remove any orphaned test in `crates/<crate>/tests/`.
3. If removing tests lowers assertion count, ensure crate still satisfies `.agents/rules/unit-testing-policy.md` ($\ge 2$ tests, $\ge 10$ assertions).

### Step 4: Quality Gate Verification
Always verify workspace health after pruning:
```powershell
cargo fmt --all
python tools/harness/pre_flight.py
python tools/harness/run_tests.py --unit
python tools/harness/run_tests.py --integration
```

---

## 3. Subagent Execution Template

When delegating dead code pruning to a subagent:
- **TaskName**: "Auditing and Pruning Dead Code: <crate_name>"
- **Prompt**:
  ```markdown
  Audit dead and zombie code for crate `<CRATE_NAME>`.
  Follow .agents/skills/remove-dead-code/SKILL.md:
  1. Run `python tools/harness/detect_dead_code.py --crate <CRATE_NAME>`.
  2. Triage dead symbols vs host boundary methods.
  3. Remove confirmed dead symbols and their orphaned tests.
  4. Run `python tools/harness/pre_flight.py`.
  5. Return structured Dead Code Pruning Report.
  ```

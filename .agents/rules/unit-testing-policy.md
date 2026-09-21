---
trigger: always_on
description: >
  Core unit testing invariants: tests/ directory placement, test_ naming prefix,
  no inline #[cfg(test)] in src/, change-coupling gate, 3-tier taxonomy labels,
  and repro-first bug mandate. Full audit playbook lives in audit-code-quality skill.
---

# Unit Testing Policy — Core Invariants

## Placement & Structure
- All tests **must** live in `crates/<crate>/tests/`, never inline in `src/`.
- Every test file **must** start with `test_` (e.g. `test_line.rs`, `test_mfm.rs`).
- Shared non-test helpers go in `tests/common/mod.rs`, not in a bare `tests/helpers.rs`.
- Multi-module crates maintain 1:1 parity: `line.rs` → `test_line.rs`.

## Change-Coupling Gate (Per-Commit)
- Every commit touching `crates/<crate>/src/` **must** also touch `crates/<crate>/tests/`.
- Enforced by `pre_flight.py --quick` → `check_test_coupling.py --staged`.
- Minimum density: ≥ 2 `#[test]` functions and ≥ 10 assertions per crate.

## 3-Tier Taxonomy
- **Tier 1 (L1):** Isolated unit tests, single crate, < 2s. `run_tests.py --unit`
- **Tier 2 (L2):** Headless multi-crate integration (machine_loop, debugger, gui). `run_tests.py --integration`
- **Tier 3 (L3):** Silicon verification — Tom Harte SingleStepTests, DMA contention, vAmigaTS. `run_tests.py --harness`

## Tier 2 Integration Trigger (4-Question Rule)
Add a `tests/test_<subsystem>_machine_integration.rs` whenever modifying a chip/peripheral that:
1. Gates execution via register bits (`DMACON`, `INTENA`, `COPCON`), OR
2. Autonomously transfers Chip RAM over multiple CCK cycles, OR
3. Raises an interrupt line or signals another chip, OR
4. Competes for Chip RAM slots / stalls the CPU.

## Repro-First Bug Mandate
Before touching production code: write a failing test → confirm red → fix → verify green.

## Dead Code & Zombie Tests
- Zero test-only zombie methods (callers only in `tests/`, zero in `src/`).
- Exception: intentional Host I/O boundary methods (keyboard, joystick, floppy, display, audio, debugger controls) — verified by `audit_code_quality.py --dead-code`.

## Full Audit Playbook
Detailed recipes (archetype harnesses, GUI headless patterns, API coverage, Host I/O symbol list, Definition of Done checklist) → [`audit-code-quality`](../skills/audit-code-quality/SKILL.md) skill and [`Testing Strategy and Quality Assurance.md`](../../Obsidian/Amiga/Design/Testing%20Strategy%20and%20Quality%20Assurance.md).

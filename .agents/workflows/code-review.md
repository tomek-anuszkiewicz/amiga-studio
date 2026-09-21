---
name: code-review
description: Comprehensive architecture, rules compliance, and code quality audit for Amiga 500 emulator tasks and milestones
---

# Workflow: Code & Specification Compliance Review

Run this only when explicitly requested by the user or before a major architectural branch merge.

**Run automated gates first** (covers ~85% of checks):
```powershell
python tools/harness/pre_flight.py --quick   # formatting, clippy, architecture rules, dead code
python tools/harness/pre_flight.py --milestone  # + condition soup, docs quality, English purity
```

Then manually inspect `git diff` for the 7 checks that cannot be automated:

---

## Manual Diff Checklist (Non-Automated)

### A. Endianness & Systems Safety
- [ ] All multi-byte guest values use explicit `from_be_bytes` / `to_be_bytes` — no implicit host-endian reinterpretation.
- [ ] ALU and cycle counter operations use wrapping arithmetic (`wrapping_add`, `wrapping_sub`) — no silent overflow.

> `transmute_ptr_to_ptr`, `cast_ptr_alignment`, `Rc`/`RefCell`/`Arc`/`Mutex` are already `deny` in `Cargo.toml` — Clippy catches them at compile time.

### B. WASM Portability (Core Crates Only)
- [ ] Zero `std::time::Instant`, `std::thread`, `std::fs` calls in `crates/cpu/`, `crates/memory_bus/`, `crates/agnus/`, `crates/denise/`, `crates/paula/`, `crates/cia/`.
  *(Clippy catches this only when cross-compiling for `wasm32`.)*

### C. Architecture Boundaries
- [ ] New subsystem state structs implement `serde::Serialize` and `serde::Deserialize` (save-state contract).

### D. Defect Retrospection (Bug Fixes & Refactors Only)
- [ ] Root cause documented: *"Why did this happen at the hardware model level?"*
- [ ] Regression test added covering the exact failure mode — committed as a permanent sentinel.
- [ ] Institutional prevention evaluated: does a Clippy lint, architecture test, or design doc update prevent this class of defect from recurring?

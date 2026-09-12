---
trigger: always_on
description: Strict constitutional size ceiling (<= 14,000 bytes) and non-redundancy policy for AGENTS.md.
---

# AGENTS.md Constitutional Non-Redundancy & Size Limit Rule

This rule governs the scope, structure, and byte limits of [`AGENTS.md`](../../AGENTS.md).

---

## 1. Constitutional Role: High-Level Index & Core Machine Invariants

[`AGENTS.md`](../../AGENTS.md) is the central architectural constitution for the emulator. It is injected into every agent prompt session. Its responsibilities are strictly limited to:

1. **Section 1 (Rules Index):** Concise, 1-line pointers linking to modularized operating rules under `.agents/rules/*.md`.
2. **Section 2 (Core Architectural Principles):** Target platforms, color clock execution model (`CCK1`/`CCK2`), decoupled ownership without circular references, and hardware circuit simulation.
3. **Section 3 (Machine Systems Invariants):** Guest vs host Big-Endianness, zero panics on guest code, wrapping arithmetic, zero runtime heap allocations in hot paths, and pointers to modular subsystem guidelines.
4. **Section 4 (Definition of Done):** Lean checklist pointing to rules and workflows.
5. **Section 5 (Knowledge Base):** Pointers to specifications, manuals, and test vectors.

---

## 2. Strict Prohibition of Content Duplication

- **Zero Rule Re-Implementation:** `AGENTS.md` must **never duplicate** verbose operational rules, full skill procedures, code-level examples, or extensive checklists that are already modularized under `.agents/rules/*.md`.
- **Reference by Pointer Only:** Whenever a new operational rule is added, add only a single 1-line bullet in Section 1 pointing to the respective `.agents/rules/<rule>.md` file. Never paste the rule's body into `AGENTS.md`.

---

## 3. Byte Budget & Automated CI Enforcement

- **Strict Size Ceiling:** `AGENTS.md` must strictly remain $\le 14,000$ bytes on disk.
- **Rationale:** Antigravity silently truncates rule files exceeding ~24,000 bytes. Keeping `AGENTS.md` under 14,000 bytes prevents prompt bloat, conserves context token budget, and eliminates truncation risk.
- **Automated Verification Gate:** Enforced on every build via `test_rule_files_size_limit_and_truncation_safety` in `crates/test_runner/tests/test_architecture_rules.rs`.

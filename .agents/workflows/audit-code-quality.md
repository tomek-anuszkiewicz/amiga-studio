---
name: audit-code-quality
description: Run full-workspace architectural code quality audit covering dead code, test zombies, visibility leaks, SRP, and design sync
---

# Workflow: Architectural Code Quality Audit & Pruning

Use this workflow to execute a comprehensive, on-demand quality audit across the entire Rust workspace, triage dead code and test zombies, enforce the Principle of Minimum Visibility, and verify specification synchronization.

---

## 1. Zero-Parameter Run (`/audit-code-quality`)
When invoked without parameters:
1. **Execute Universal Quality Audit:**
   ```powershell
   python tools/harness/audit_code_quality.py --all
   ```
2. **Execute Pre-Flight Quality Gate:**
   ```powershell
   python tools/harness/pre_flight.py
   ```
3. **Execute Architecture Rules:**
   ```powershell
   cargo test -p test_runner --test test_architecture_rules -- --quiet
   ```

---

## 2. Targeted Audit Commands
- **Audit Dead Code & Test Zombies:**
  ```powershell
  python tools/harness/audit_code_quality.py --dead-code
  ```
- **Audit Visibility Leaks (`pub` vs `pub(crate)` vs private):**
  ```powershell
  python tools/harness/audit_code_quality.py --visibility
  ```
- **Audit SRP & 800-Line File Ceilings:**
  ```powershell
  python tools/harness/audit_code_quality.py --srp
  ```
- **Audit Design Specifications Drift:**
  ```powershell
  python tools/harness/audit_code_quality.py --design-sync
  ```
- **Audit Skills Catalog & Script Locality:**
  ```powershell
  python tools/harness/audit_code_quality.py --skills --scripts
  ```
- **Audit Workflow-Skill Symmetry & Rule Coverage:**
  ```powershell
  python tools/harness/audit_code_quality.py --governance
  ```

---

## 3. Remediation Procedure
Follow the detailed playbooks in `.agents/skills/audit-code-quality/SKILL.md`:
1. **Test-Only Zombies:** Distinguish external Host I/O boundaries from dead internal scaffolding. Prune dead symbols and orphaned test cases.
2. **Visibility Demotion:** Demote over-exposed symbols to `pub(crate)` or private `fn`.
3. **SRP Decompositions:** Decompose oversized files (> 800 lines) into submodules using `.agents/skills/refactor-split-module/SKILL.md`.
4. **Design Sync:** Update drifted specifications and bump checkpoint commits via `--design-bump <doc>`.
5. **Workflow & Skill Governance:** Promote identified milestone skills to first-class `.agents/workflows/<name>.md` workflows, and ensure active rules maintain backing skills.

---

## 4. Output Contract
Conclude with the standardized summary report:
```markdown
### 🛡️ Code Quality Audit & Pruning Report
- **Scope Scanned:** Workspace (all crates)
- **Dead Code Pruned:** <count> symbols
- **Test-Only Zombies Handled:** <count> retained (Host I/O) / <count> pruned
- **Visibility Demoted:** <count> symbols (`pub` -> `pub(crate)` / private)
- **SRP / Cohesion Decompositions:** <count> files/structs
- **Skills Catalog Sync:** [PASS | <count> discrepancies]
- **Script Locality & Governance:** [PASS | <count> anomalies]
- **Workflow & Skill Governance:** [PASS | <count> issues (<count> candidates)]
- **Design Specs Sync:** [PASS | <count> drifted]
- **Verification:** `pre_flight.py` (PASS), `cargo test` (PASS)
```

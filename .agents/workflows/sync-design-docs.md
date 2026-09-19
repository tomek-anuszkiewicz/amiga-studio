---
name: sync-design-docs
description: Audit and synchronize Obsidian design specifications with active Rust code commits and bump checkpoints
---

# Workflow: Synchronize Design Documentation

Use this workflow to audit code-backed design specifications in `Obsidian/Amiga/Design/`, inspect diffs against active Rust code commits, update architectural documentation, and bump synchronization checkpoints.

---

## 1. Zero-Parameter Run (`/sync-design-docs`)
When invoked without parameters:
1. **Detect Code-Documentation Drift:**
   ```powershell
   python tools/harness/audit_code_quality.py --design-sync
   ```
2. **Inspect Diffs for Drifted Specifications:**
   For each drifted document reported:
   ```powershell
   python tools/harness/audit_code_quality.py --design-diff <doc_name>
   ```
3. **Synchronize Specification Content:**
   - Update register bitfields, timing constants, clock phases (`CCK1`/`CCK2`), and bus arbitration rules.
   - Prune obsolete draft code or speculative pseudo-code.
   - Maintain dual-layer linking and inverted pyramid hierarchy.
4. **Bump Checkpoint to HEAD:**
   Once verified or updated:
   ```powershell
   python tools/harness/audit_code_quality.py --design-bump <doc_name>
   ```
5. **Verify Quality & Link Integrity:**
   ```powershell
   python tools/harness/pre_flight.py
   cargo test -p test_runner --test test_architecture_rules -- --quiet
   ```

---

## 2. Targeted Commands
- **Audit Drift Status:**
  ```powershell
  python tools/harness/audit_code_quality.py --design-sync
  ```
- **Inspect Specific Spec Diff:**
  ```powershell
  python tools/harness/audit_code_quality.py --design-diff Denise.md
  ```
- **Bump Verified Spec Checkpoint:**
  ```powershell
  python tools/harness/audit_code_quality.py --design-bump Denise.md
  ```

---

## 3. Execution Runbook
Follow the operational procedure in [`.agents/skills/sync-design-docs/SKILL.md`](../skills/sync-design-docs/SKILL.md):
1. **Audit:** Run `--design-sync` to list specifications behind HEAD.
2. **Inspect:** Run `--design-diff <doc>` to view exact Rust crate commits.
3. **Update:** Modify markdown under `Obsidian/Amiga/Design/` ensuring exhaustive technical depth.
4. **Prune:** Eliminate raw code duplication and speculative snippets.
5. **Checkpoint:** Stamp current HEAD commit via `--design-bump <doc>`.

---

## 4. Output Contract
Conclude with the standardized summary report:
```markdown
### 📐 Design Documentation Sync Report
- **Total Specifications Tracked:** <count> docs
- **Drifted Specifications Reconciled:** <count> docs
- **Checkpoint Stamped to HEAD:** <head_commit>
- **Files Modified:** <list of updated markdown files>
- **Link & Architecture Integrity:** `test_architecture_rules` (PASS)
```

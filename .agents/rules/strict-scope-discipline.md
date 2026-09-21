---
trigger: always_on
description: Strict scope discipline, minimal diffs, task containment, zero unsolicited refactoring or defect expansion, and mandatory delivered vs suggested reporting.
---

# Strict Scope Discipline & Anti-Scope Creep Rule

This rule governs all agent pair-programming interactions, defect resolutions, refactorings, and feature additions across the workspace. It enforces strict task boundaries and prevents unsolicited scope expansion.

---

## 1. The Core Mandate: Task Containment & Minimal Diffs

When instructed to execute a specific task, bug fix, or modification:
1. **Strict Task Perimeter:**
   - Execute strictly the work required to satisfy the user's explicit prompt.
   - Never expand the scope to touch adjacent functions, sibling opcodes, or unrelated subsystems without explicit authorization.
2. **Minimal Necessary Diff Principle:**
   - Modify the minimal set of files and lines needed to accomplish the task, satisfy compiler checks, and pass pre-commit / change-coupling gates.
   - Avoid "drive-by refactorings" or stylistic touch-ups in code that is already functioning correctly.
3. **Escalate, Never Execute Unsolicited Work:**
   - If you spot code smells, dead code, architectural inconsistencies, adjacent bugs, or optimization opportunities while working on the target task, **do NOT modify them unsolicited**.
   - Instead, capture them and present them to the user under the mandatory recommendations section at the end of the turn.

---

## 2. Prohibited Scope Creep Patterns

### A. Sibling Opcode & Multi-Instruction Creep
- **Forbidden:** When asked to fix or adjust instruction `X` (e.g. `TRAPV`), unilaterally inspecting and modifying sibling instructions (`CHK`, `ILLEGAL`, `TRAP`) because they share similar micro-steps or exception vectors.
- **Required:** Fix only instruction `X`. If siblings suffer from identical silicon defects, solve the shared helper or highlight them under recommendations for the user to schedule.

### B. Drive-By Code Cleanup & Aesthetic Rewrites
- **Forbidden:** While editing `foo.rs` to fix a calculation, noticing adjacent functions with awkward conditionals or deprecated idioms and rewriting them "while in the area".
- **Required:** Leave adjacent code untouched. Focus the diff entirely on the requested modification.

### C. Unsolicited API & Architecture Cascades
- **Forbidden:** Changing an internal method signature or data structure and cascading those changes across unaffected subsystems when a localized, minimal solution was requested.
- **Required:** Keep the change surface as tight and local as possible.

---

## 3. Harmonization with Structural Root-Cause Resolution

The mandate in [`structural-root-cause.md`](structural-root-cause.md) ("resolve root cause upstream rather than masking symptoms with coordinate nudges or ad-hoc regexes") governs the **physical and causal fidelity of the mechanism**, NOT the spatial scope of the task:
- **Mechanism Fidelity (Mandatory):** Never use a $\pm 1$ coordinate nudge, magic cycle offset, or ad-hoc `if` branch to silence a bug. Trace the genuine clock phase or state machine bug.
- **Task Scope (Strictly Contained):** Resolving the causal mechanism does not authorize rewriting unrelated or working siblings. If the upstream engine fix naturally fixes all consumers without extra code, that is correct; if additional code changes are needed across other subsystems, escalate those to the user.

---

## 4. Mandatory Turn Completion Reporting Format

Every response concluding an execution task must end with a structured breakdown distinguishing delivered changes from observed future opportunities:

```markdown
### 🎯 Delivered Changes
- [Concrete file or component modified]: [Concise 1-line summary of what was implemented]
- [Test or validation]: [Summary of test suites executed and passing]

### 💡 Observed Opportunities & Future Recommendations
- [Observed issue, smell, or potential extension spotted during work]
- [Candidate future refactoring or test coverage enhancement]
```

If no adjacent opportunities were spotted, explicitly state:
`- None observed; scope remained 100% contained.`

---

## 5. Pre-Commit Scope Conscience Check

Before committing changes, review the git diff against this checklist:
- [ ] Are all modified files directly required to fulfill the user's prompt or pass CI gates?
- [ ] Is the changeset free of drive-by refactorings or stylistic edits in untouched functions?
- [ ] Were any adjacent issues spotted during work escalated as suggestions rather than executed unsolicited?
- [ ] Does the final response include both **Delivered Changes** and **Observed Opportunities & Future Recommendations**?

---
trigger: always_on
description: Mandatory structural root-cause resolution; strict prohibition of local symptom patches (pixel nudging, ad-hoc regexes, special-case branches).
---

# Structural Root-Cause Resolution Rule (Zero Local Symptom Patches)

---

## 1. Core Mandate

When addressing any defect, test divergence, or user-reported anomaly:
- **Never apply surface-level local tweaks to silence the symptom.**
- Trace the data lifecycle or hardware timing upstream. Implement a structural solution that resolves the root cause for the entire class of problems.

---

## 2. Prohibited Anti-Patterns

### A. Coordinate & Timing Nudging
- **Forbidden:** Adjusting an offset, beam coordinate, or cycle delay by $\pm 1$ / $\pm 2$ to make a test pass.
- **Required:** Trace the physical silicon clock phase or signal origin — determine *why* the event fired early or late.

### B. Ad-Hoc Regexes & String Hacks
- **Forbidden:** Hardcoded `re.sub(r"HARDW\s+ARE", "HARDWARE")` or character-stripping to fix a single OCR artifact.
- **Required:** Fix pipeline ordering upstream so clean data is produced before emission.

### C. Isolated Special-Casing
- **Forbidden:** Special-case `if` branches for a single opcode, register address, or file name.
- **Required:** Modify the state machine or data model to handle the condition generically.

---

## 3. Assume Systematic Scope (for the Mechanism, Not the Spatial Scope)

- One broken chapter title → same OCR flaw affects all headings. One shifted raster line → arbitration logic affects all lines.
- Fix the shared upstream mechanism, not just the reported instance.
- **Not a license for spatial expansion:** if the fix requires touching modules outside the task perimeter, escalate to the user per [`strict-scope-discipline.md`](strict-scope-discipline.md).

---

## 4. Pre-Submission Checklist

- [ ] Free of $\pm 1$ coordinate/cycle nudges without silicon timing proof?
- [ ] Free of hardcoded ad-hoc string substitutions or regex patches?
- [ ] Resolves the entire class of errors, not just the reported instance?
- [ ] Root cause addressed upstream in the data lifecycle or state machine?

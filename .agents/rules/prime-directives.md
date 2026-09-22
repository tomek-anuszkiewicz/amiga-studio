---
trigger: always_on
description: Prime Directives on thinking mode and code modifications (invariant modeling over hardcoded fixes, explicit request before modification, and exhaustive search before declaring).
---

# Prime Directives: Thinking Mode & Code Modifications

This rule defines foundational engineering invariants governing how all code modifications, bug fixes, refactorings, and architectural designs must be approached across the repository.

---

## 1. Generalization over Hardcoded Fixes (No Overfitting)

### A. Model the Invariant, Not the Instance
- Strictly prohibit fixes that rely on transient data anomalies (e.g. matching against specific PDF filenames, string literals, hardcoded paths, or one-off regex hacks).
- If processing documents, parsers, or memory buffers, identify the underlying format, header, or structural invariant. Design solutions that work conceptually across the entire data category, not just the single file triggering the bug.

### B. Root-Cause Conceptualization
- Before writing code, articulate the architectural mechanism causing the failure. If the proposed fix only works because of the current input's naming or layout quirks, reject it immediately.

---

## 2. Single-Point Verification & Minimal Blast Radius

### A. Prove Before Scaling
- **NEVER** execute speculative multi-file refactors or edits across dozens of locations simultaneously without first isolating the core fix.
- Locate the single authoritative upstream choke point. Apply the conceptual fix there first.

### B. Verify with Isolated Repro
- Validate that the single-point change solves the problem cleanly before propagating changes or modifying downstream call sites.

### C. No Premature Mass Edits
- If a proposed fix touches more than 2–3 files for a localized issue, halt. Re-evaluate whether an abstraction layer or single shared utility function can resolve the issue without spreading changes across the codebase.

---

## 3. Exhaustive Search Before Modifying (No Duplicate Code)

### A. Search Before Declaring
- **NEVER** introduce a new constant, static, helper function, enum, or struct without first searching the workspace (`rg` / AST).
- If an equivalent symbol exists, import and reuse it. Never duplicate definitions to satisfy a quick fix.

---

## 4. Explicit Request Before Modification (Zero Unsolicited Code Changes)

### A. Strict Prohibition of Premature or Unsolicited Edits
- **NEVER begin modifying code, editing files, or executing refactorings unless the USER directly and explicitly asks you to do so.**
- When the user asks investigatory questions, reports symptoms, discusses ideas, or asks for root-cause explanations:
  - Confine actions strictly to analysis, inspection, explaining architectural mechanisms, and proposing designs or plans.
  - Do not jump ahead into modifying source files or applying fixes on your own initiative without explicit user direction.

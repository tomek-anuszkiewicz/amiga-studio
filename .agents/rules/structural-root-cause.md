# Scope and Root-Cause Resolution

- Inspection questions authorize investigation and explanation. Modify files when implementation is requested or proposed changes are accepted.
- Complete the requested change, including affected consumers, tests, and documentation. Avoid unrelated cleanup; obtain direction if the cause requires expanding the agreed scope.
- Explain the failing data flow, clock phase, or state transition before repairing it. Follow [repro-first.md](repro-first.md) to confirm the defect and fix.
- Repair a shared upstream mechanism only when evidence establishes it. Reuse authoritative constants, types, and helpers; prove the mechanism on an isolated case before scaling.
- Do not nudge coordinates or timing to fit a test, hardcode one-artifact OCR substitutions, or special-case an opcode, address, or filename to mask a defect. Represent the actual condition in the state machine or data model.
- Before completion, verify scope, cause, and absence of duplicated definitions; requested refactors follow [clean-break-refactoring.md](clean-break-refactoring.md). Report delivered work, actual verification, and material unresolved findings.

## Execution-Plan Lifecycle

- Keep `.agent/tasks/` plans while their tasks are active, paused, or blocked. Plans are working instructions, not the permanent engineering record.
- On verified completion or explicit user-directed closure, preserve the stable task ID, delivered scope, decisions, actual verification, and remaining limitations in the corresponding DIARY.md entry and Git history. Transfer still-required future work to the roadmap or another active plan; preserve any explicit user decision to waive or close that scope.
- After the closure record is committed, delete the closed task's plan automatically as part of the same closing workflow, without asking for separate cleanup confirmation. Remove only plans belonging to that closed task; retain unrelated or unfinished plans.
- User-directed closure and plan deletion do not turn failed or unrun checks into passing results. Keep those limits in the durable closure record.

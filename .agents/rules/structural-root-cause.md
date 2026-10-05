# Scope and Root-Cause Resolution

- Inspection questions authorize investigation and explanation. Modify files when implementation is requested or proposed changes are accepted.
- Complete the requested change, including affected consumers, tests, and documentation. Avoid unrelated cleanup; obtain direction if the cause requires expanding the agreed scope.
- Explain the failing data flow, clock phase, or state transition before repairing it. Follow [repro-first.md](repro-first.md) to confirm the defect and fix.
- Repair a shared upstream mechanism only when evidence establishes it. Reuse authoritative constants, types, and helpers; prove the mechanism on an isolated case before scaling.
- Do not nudge coordinates or timing to fit a test, hardcode one-artifact OCR substitutions, or special-case an opcode, address, or filename to mask a defect. Represent the actual condition in the state machine or data model.
- Before completion, verify scope, cause, and absence of duplicated definitions; requested refactors follow [clean-break-refactoring.md](clean-break-refactoring.md). Report delivered work, actual verification, and material unresolved findings.

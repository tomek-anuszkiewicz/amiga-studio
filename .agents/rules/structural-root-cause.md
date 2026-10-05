# Scoped Changes and Structural Root-Cause Resolution

## Task Scope

- Investigatory questions authorize inspection and explanations. Modify files when the user requests implementation or accepts the proposed changes.
- Make the changes needed to complete the task, including affected callers, tests, and documentation. Avoid unrelated cleanup or sibling feature work.
- If resolving the cause requires work outside the agreed task, explain the dependency and obtain direction before expanding scope.
- Report delivered changes, verification, and material unresolved findings. Use a response format appropriate to the task; empty recommendations sections are unnecessary.

## Fix the Mechanism

- Explain the failing data flow, clock phase, or state transition before changing it. Use the [repro-first procedure](repro-first.md) to confirm the defect and the fix.
- Fix the shared upstream mechanism when evidence identifies one. Do not assume unrelated failures have a single cause.
- Search for existing constants, types, and helpers before introducing equivalents; reuse the authoritative definition.
- Prove the mechanism on an isolated case before scaling the change. File count alone is not a reason to stop a necessary repair.

## Prohibited Symptom Patches

- **Coordinate and timing nudges:** Do not add $\pm 1$ / $\pm 2$ offsets merely to make a test pass. Establish the physical phase or signal origin.
- **OCR string hacks:** Do not hardcode substitutions such as `re.sub(r"HARDW\s+ARE", "HARDWARE")` for one artifact. Correct the shared processing mechanism.
- **Isolated special-casing:** Special-case `if` branches for a single opcode, register address, or file name are forbidden. Modify the state machine or data model to handle the condition generically.

Before completion, verify that the diff addresses the cause, stays within the agreed scope, and adds no unrelated cleanup or duplicated definitions. For a requested refactor, update all affected consumers according to [clean-break-refactoring.md](clean-break-refactoring.md).

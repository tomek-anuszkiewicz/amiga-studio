# Scope and Root-Cause Resolution

- Inspection questions authorize investigation and explanation. Modify files when implementation is requested or proposed changes are accepted.
- Every new behavior must follow from a user requirement or be technically necessary for that requirement to work correctly. Explain necessity in terms of what would fail or remain incomplete without it. Convenience, possible future use, and agent preference do not establish necessity.
- Complete the requested change, including affected consumers, relevant tests, and documentation. Choose routine implementation details autonomously within that scope. Synchronize design documentation for code changes before an authorized commit, unless documentation work was explicitly requested earlier. Avoid unrelated cleanup; obtain direction if the cause requires expanding the agreed scope.
- Present optional features, flags, configuration, automation, and fallback behavior as proposals; implement them only after acceptance. Leave product and editorial decisions, such as selecting, hiding, or replacing source content, to the user unless explicitly delegated. Continue authorized work that does not depend on those decisions.

## Defect Resolution

- When investigating a suspected defect, trace the failing data flow, clock phase, or state transition toward its cause before choosing a repair.
- Before changing production behavior to fix a defect, identify or author a focused regression test and confirm that it fails on the original code for the expected reason. An existing isolated failing test is sufficient. Follow [repro-first.md](repro-first.md) for the detailed procedure.
- Repair a shared upstream mechanism only when evidence establishes it. Reuse authoritative constants, types, and helpers; prove the mechanism on an isolated case before scaling.
- Repair the demonstrated mechanism within the requested scope, then confirm the reproduction passes and run adjacent domain regressions.
- Keep the regression protection after the fix; never delete or disable it merely to hide a failure. Rust test placement follows [unit-testing-policy.md](unit-testing-policy.md).
- Do not nudge coordinates or timing to fit a test, hardcode one-artifact OCR substitutions, or special-case an opcode, address, or filename to mask a defect. Represent the actual condition in the state machine or data model.
- Apply [spec-compliance.md](spec-compliance.md); never adjust goldens or timing offsets to manufacture a pass.
- Use the focused reproduction procedure in [test-runner](../skills/test-runner/SKILL.md). For CPU silicon divergences, use [m68k-singlestep-test](../skills/m68k-singlestep-test/SKILL.md).
- Before completion, review the diff and identify the requirement or technical necessity behind each added behavior. Remove unrequested additions introduced by the current task; preserve pre-existing work. Verify cause and absence of duplicated definitions; requested refactors follow [clean-break-refactoring.md](clean-break-refactoring.md). Report delivered work, actual verification, and material unresolved findings.

Ordinary defect completion requires relevant focused checks. Full commit and milestone profiles run only at their respective triggers in [AGENTS.md](../../AGENTS.md); report actual results and remaining gaps.

## Execution-Plan Lifecycle

- Keep `.agent/tasks/` plans while their tasks are active, paused, or blocked. Plans are working instructions, not the permanent engineering record.
- On verified completion or explicit user-directed closure, preserve the stable task ID, delivered scope, decisions, actual verification, and remaining limitations in the corresponding DIARY.md entry and Git history at the next commit required by [commit policy](git-commits.md). Retain the plan until that closure record is committed. Transfer still-required future work to the roadmap or another active plan; preserve any explicit user decision to waive or close that scope.
- After the closure record is committed, delete the closed task's plan automatically as part of the same closing workflow, without asking for separate cleanup confirmation. Remove only plans belonging to that closed task; retain unrelated or unfinished plans.
- User-directed closure and plan deletion do not turn failed or unrun checks into passing results. Keep those limits in the durable closure record.

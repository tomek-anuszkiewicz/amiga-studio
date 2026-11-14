# Scope and Root-Cause Resolution

- Inspection questions authorize investigation and explanation. Modify files when implementation is requested or proposed changes are accepted.
- Every new behavior must follow from a user requirement or be technically necessary for that requirement to work correctly. Explain necessity in terms of what would fail or remain incomplete without it. Convenience, possible future use, and agent preference do not establish necessity.
- Complete the requested change, including affected consumers, relevant tests, and documentation. Choose routine implementation details autonomously within that scope. Review specification compliance and enrich compatible implementation details during batch preparation, unless documentation work was explicitly requested earlier. Follow [spec-compliance.md](spec-compliance.md) for the enrichment boundary and conflicts. Avoid unrelated cleanup; obtain direction if the cause requires expanding the agreed scope.
- Present optional features, flags, configuration, automation, and fallback behavior as proposals; implement them only after acceptance. Leave product and editorial decisions, such as selecting, hiding, or replacing source content, to the user unless explicitly delegated. Continue authorized work that does not depend on those decisions.

## Defect Resolution

- When investigating a suspected defect, trace the failing data flow, clock phase, or state transition toward its cause before choosing a repair.
- Before changing production behavior, confirm the defect on the original code using a focused test, reproducible case, execution trace, or measurement suited to the failure. Choose the diagnostic method autonomously; distinguish established causes from hypotheses.
- Repair the demonstrated mechanism within the requested scope. Change a shared upstream mechanism only when evidence identifies it as the cause; reuse authoritative constants, types, and helpers.
- Verify the repaired behavior against the original reproduction or measurement and run relevant adjacent regressions. Keep repeatable regression protection suited to the defect; never delete or disable it merely to hide a failure. Test selection and placement follow [unit-testing-policy.md](unit-testing-policy.md).
- Do not nudge coordinates or timing to fit a test, hardcode one-artifact OCR substitutions, or special-case an opcode, address, or filename to mask a defect. Represent the actual condition in the state machine or data model.
- Apply [spec-compliance.md](spec-compliance.md); never adjust goldens or timing offsets to manufacture a pass.
- Use the [verification reference](../../tools/verification/README.md) for focused test commands and saved reports. For CPU vector divergences, also consult the [CPU verification specification](../../Obsidian/Amiga/Design/Verification/CPU%20SingleStepTests.md).
- Before completion, review the diff and identify the requirement or technical necessity behind each added behavior. Remove unrequested additions introduced by the current task; preserve pre-existing work. Verify cause and absence of duplicated definitions; requested refactors follow [Compatibility and Internal Refactoring](../../AGENTS.md#compatibility-and-internal-refactoring). Report delivered work, actual verification, and material unresolved findings.

Ordinary defect completion requires relevant focused checks. Reuse still-valid context and evidence within the session; a turn alone triggers no language scan, coverage campaign, graph refresh or broad audit. Report implementation separately from verified batch readiness and roadmap closure. Full commit and milestone profiles run only at their respective triggers in [AGENTS.md](../../AGENTS.md); report actual results and remaining gaps.

## Execution-Plan Lifecycle

- Keep `.agent/tasks/` plans while their tasks are active, paused, or blocked. Plans are working instructions, not the permanent engineering record.
- On verified completion or explicit user-directed closure, preserve the stable task ID, delivered scope, decisions, actual verification, and remaining limitations in the corresponding DIARY.md entry and Git history at the next commit required by [commit policy](git-commits.md). Retain the plan until that closure record is committed. Transfer still-required future work to the roadmap or another active plan; preserve any explicit user decision to waive or close that scope.
- After the closure record is committed, delete the closed task's plan automatically as part of the same closing workflow, without asking for separate cleanup confirmation. Remove only plans belonging to that closed task; retain unrelated or unfinished plans.
- User-directed closure and plan deletion do not turn failed or unrun checks into passing results. Keep those limits in the durable closure record.

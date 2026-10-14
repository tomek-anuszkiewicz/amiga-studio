---
name: code-review
description: Independently review a completed change against repository rules, specifications, and available verification evidence when a review is requested or a major delivery needs one.
---

# Independent Code Review

Use this skill for a requested double-check, a completed roadmap milestone or substantial feature PR, or an assigned reviewer task. Ordinary edits and commits do not invoke it. A review-only request authorizes inspection and a report, not file changes, staging, or a commit.

## Establish the Reviewed Scope

Identify the exact patch or revision under review, its intended behavior, and the affected domains. Include explicitly selected new files; `git diff` alone does not show untracked files. For a proposed commit, use the intended snapshot described in [sync-design-docs](../sync-design-docs/SKILL.md#1-establish-the-intended-snapshot). State any paths or behavior outside the reviewed scope.

Read [AGENTS.md](../../../AGENTS.md) and the rules it routes to for the affected domains. Use the current rule text as the source of requirements. Do not reproduce its detailed checklists here or treat an example in this skill as a new requirement.

For a review that is not preparing an authorized commit, assess known specification requirements and encountered conflicts, but leave code/design synchronization and parity review to the pre-commit or milestone workflow unless explicitly requested.

## Check Whether the Rules Were Met

For each applicable rule group, ask whether the reviewed change and available evidence demonstrate compliance. Cite the concrete changed code, test, specification, or verification result behind each material conclusion. Mark an obligation **unverified** when the needed evidence is absent; a passing check establishes only what that check actually covers.

| Rule owner | Review question |
| --- | --- |
| [Scope and root cause](../../rules/structural-root-cause.md), [specification compliance](../../rules/spec-compliance.md) | Does the change address the demonstrated mechanism without an ad-hoc workaround or an unsupported change to protected references? Were specification conflicts handled as required? |
| [Rust practices](../../rules/rust-best-practices.md), [performance and readability](../../rules/performance-and-readability.md), [file cohesion](../../rules/file-size-and-cohesion.md), [workspace boundaries](../../rules/workspace-structure-and-reexports.md), [clean refactoring](../../rules/clean-break-refactoring.md) | Do the implementation and affected callers follow the applicable code and architecture rules? Inspect behavior and maintainability beyond the mechanical checks. |
| [Testing policy](../../rules/unit-testing-policy.md) | Do existing or changed tests protect the relevant behavior and failure modes, including integration boundaries where applicable? Identify a demonstrated gap rather than requiring a test per source file. |
| [Documentation maintenance](../../rules/docs-maintenance.md), [diary](../../rules/diary-maintenance.md), [roadmap](../../rules/roadmap-maintenance.md) | For the actual delivery stage, were affected specifications, completion records, and remaining backlog handled as required? |
| [Language policy](../../rules/language-policy.md), [path privacy](../../rules/no-external-paths.md) | Are repository text and paths compliant within the reviewed change? Distinguish scanner findings from confirmed language violations. |
| [Atomic commits](../../rules/git-commits.md) | If a commit is under review, does its intended snapshot, verification, documentation review, diary entry, and staging satisfy the commit procedure? |

Apply other domain rules routed by [AGENTS.md](../../../AGENTS.md) when relevant, especially CPU opcode, chip/bus, GUI, and PowerShell rules. For hardware or regression review, use the reference evidence required by [specification compliance](../../rules/spec-compliance.md). For a milestone, check its completion requirements through [roadmap maintenance](../../rules/roadmap-maintenance.md). Do not infer that every conditional rule applies to every patch.

## Review Beyond Mechanical Gates

- Trace the changed behavior through its callers, state transitions, and boundaries. Look for unintended effects and assumptions that the tests or static checks do not exercise.
- Compare claims of correctness with the actual test fixture, reference, and observed result. Distinguish isolated behavior from production integration and a static audit from behavioral proof.
- Check whether a passing gate was run on the reviewed snapshot. Separate pre-existing failures from regressions and identify checks that were not run or whose results are stale.
- Read relevant results from `python tools/verification/verify.py commit` or `milestone` when those profiles apply. Their owning workflow selects and runs the gates; do not rerun formatting, Clippy, architecture, or other completion checks solely because this skill was invoked. If evidence is missing, report the gap rather than claiming a pass. Invoke [audit-code-quality](../audit-code-quality/SKILL.md) only under its own trigger, not automatically as part of this review.

## Report

Lead with actionable findings, ordered by impact. For each finding, give the affected path and location, the violated rule or observed behavior, and the evidence. Then summarize:

- Reviewed patch or revision and any scope limits.
- Applicable rule groups: **satisfied**, **violated**, **unverified**, or **not applicable**, with brief evidence or a reason.
- Verification actually observed, including snapshot freshness, failures, and checks not run.
- Verdict: **APPROVED** only when applicable requirements are met and required evidence is current; otherwise **CHANGES REQUESTED** with concrete next actions. Do not present an unchecked template as a passed audit.

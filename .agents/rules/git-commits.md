# Atomic Commits

- Create a Git commit only on explicit user request or when closing a completed minor roadmap point (for example, 1.1 or 1.2). Ordinary implementation tasks, fixes, reviews, and documentation changes do not authorize staging or committing by themselves.
- Validate completed changes and report their results even when leaving them uncommitted. Do not label a routine task as a roadmap completion to trigger a commit.
- Commit when closing a major milestone. Outside required roadmap completion, the user controls commit timing.
- Group implementation, relevant tests, design updates, and the diary entry for one concern into one atomic commit. Do not split a feature by file category.
- Inspect status and diffs, then stage intended paths explicitly. Never blindly stage unrelated changes with `git add -A`.
- Use an English Conventional Commit message with a concise imperative summary, rationale when useful, and actual validation results. Templates are in [agent maintenance](../references/agent-tooling.md#recording-and-validating-changes).
- Every commit includes its own diary entry per [diary-maintenance.md](diary-maintenance.md).

Before each authorized commit, execute [sync-design-docs](../skills/sync-design-docs/SKILL.md) on the intended snapshot, include necessary documentation corrections, and leave already accurate documents untouched. This requirement applies even when no documentation edits result.

Append the required DIARY.md entry with work completed and verification results already observed. Finish source and documentation edits, check the intended paths and generated artifacts, and resolve index or line-ending issues before running the full commit profile. Record the profile's outcome in the commit message and final report; do not edit the diary solely to repeat that outcome. If a later correction changes the intended source, rerun the profile on the corrected source.

Then run the canonical verification entry point on the final intended source and explicitly stage the intended files. Verification does not require staging first:

```powershell
python tools/verification/verify.py commit
```

Commit only if this command exits successfully and every selected check passes on the final intended source. Any failed check, including an unrelated test or a local environment failure, stops the commit workflow.

Diagnose and resolve the failure, then rerun the full commit profile on the final intended source before committing. When resolution needs a user decision, present the findings and concrete choices in an interactive dialog when available. Keep the authorized commit workflow active while awaiting the answer. For English findings, follow [language-policy.md](language-policy.md): do not change text or vocabulary before the user decides. After the decision, apply it and continue verification and the same authorized commit without requesting commit permission again.

If the failure cannot be resolved, report blocked verification and leave the changes uncommitted. A focused passing test or a written explanation of the failure does not waive this gate. For milestone completion, the required milestone profile must also pass before its completion commit.

`verify.py` owns test selection and includes the architecture gate. Do not replace it with individually chosen completion suites. Milestone requirements are in [roadmap-maintenance.md](roadmap-maintenance.md). Run [audit-code-quality](../skills/audit-code-quality/SKILL.md) only on explicit request or before a major branch merge.

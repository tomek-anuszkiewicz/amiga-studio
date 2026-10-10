# Atomic Commits

- Create a Git commit only on explicit user request or when closing a completed minor roadmap point (for example, 1.1 or 1.2). Ordinary implementation tasks, fixes, reviews, and documentation changes do not authorize staging or committing by themselves.
- Validate completed changes and report their results even when leaving them uncommitted. Do not label a routine task as a roadmap completion to trigger a commit.
- Group implementation, relevant tests, design updates, and the diary entry for one concern into one atomic commit. Do not split a feature by file category.
- Inspect status and diffs, then stage intended paths explicitly. Never blindly stage unrelated changes with `git add -A`.
- Use an English Conventional Commit message with a concise imperative summary, rationale when useful, and actual validation results. Templates are in [agent maintenance](../../docs/ai_agents.md#recording-and-validating-changes).
- Every commit includes its own diary entry per [diary-maintenance.md](diary-maintenance.md).

Before each authorized commit, execute [sync-design-docs](../skills/sync-design-docs/SKILL.md) on the intended snapshot, include necessary documentation corrections, and leave already accurate documents untouched. This requirement applies even when no documentation edits result.

Then run the canonical verification entry point on the final intended source and explicitly stage the intended files. Verification does not require staging first:

```powershell
python tests/verify.py --commit
```

`verify.py` owns test selection and includes the architecture gate. Do not replace it with individually chosen completion suites. Milestone requirements are in [roadmap-maintenance.md](roadmap-maintenance.md). Run [audit-code-quality](../skills/audit-code-quality/SKILL.md) only on explicit request or before a major branch merge.

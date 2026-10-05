# Atomic Commits

- Commit every completed discrete implementation task before ending the turn. Inspection-only questions do not require file changes or commits.
- Group implementation, relevant tests, design updates, and the diary entry for one concern into one atomic commit. Do not split a feature by file category.
- Inspect status and diffs, then stage intended paths explicitly. Never blindly stage unrelated changes with `git add -A`.
- Use an English Conventional Commit message with a concise imperative summary, rationale when useful, and actual validation results. Templates are in [agent maintenance](../../docs/ai_agents.md#recording-and-validating-changes).
- Every commit includes its own diary entry per [diary-maintenance.md](diary-maintenance.md).

Before each commit, run these explicit gates and relevant domain tests:

```powershell
python tools/harness/pre_flight.py --quick
cargo test -p test_runner --test test_architecture_rules -- --quiet
```

Milestone requirements are in [roadmap-maintenance.md](roadmap-maintenance.md). Run [audit-code-quality](../skills/audit-code-quality/SKILL.md) only on explicit request or before a major branch merge.

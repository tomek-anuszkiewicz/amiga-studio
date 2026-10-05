# Engineering Diary

- Every commit includes a new [DIARY.md](../../DIARY.md) Section 10 entry staged with that change, including documentation, rules, tests, and merges.
- Record affected components, changes, rationale, and actual verification. Identify failures and checks not run; a commit alone does not prove milestone completion.
- Use the diary appender and timestamp format documented in [agent maintenance](../../docs/ai_agents.md#recording-and-validating-changes), rather than reading the whole diary.
- After a minor roadmap point or major milestone passes its gates, use [compact-diary](../skills/compact-diary/SKILL.md), regardless of diary size.
- Preserve decisions, alternatives, hardware findings, and evidence in separate milestone digests; retain granular entries for unfinished work. The completion commit gets one entry covering completion and compaction.

Commit validation belongs to [git-commits.md](git-commits.md); milestone obligations belong to [roadmap-maintenance.md](roadmap-maintenance.md).

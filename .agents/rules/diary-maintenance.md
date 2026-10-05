# Engineering Diary: Every Commit, Compaction at Milestones

## Per-Commit Logging

Every repository commit must include a new entry in [DIARY.md](../../DIARY.md) Section 10 describing that commit. This includes fixes, refactors, tests, rules, documentation, and merge commits. Stage the entry with the changes it records; do not postpone logging until a milestone or create a separate trailing diary commit.

Keep routine entries concise. Record affected components, what changed, why, and the actual verification results, including checks not run or known failures. A commit is not evidence that a roadmap milestone is complete.

Use the appender rather than reading the entire diary:

```powershell
python tools/harness/log_diary.py `
  --title "<Commit-sized change>" `
  --subsystems "<relative components>" `
  --changes "<change 1>; <change 2>" `
  --rationale "<reason or trade-off>" `
  --results "<observed verification>"
```

Entries use `### [YYYY-MM-DD HH:MM CET/CEST] — <Title>`. Use `--date` when an explicit local timestamp is needed. Do not claim a test passed before its result is available.

## Milestone Compaction

After a minor roadmap point or major milestone passes its completion gates, use [compact-diary](../skills/compact-diary/SKILL.md) to consolidate its settled commit entries into a milestone digest. This is the compaction trigger, regardless of diary size.

Preserve decisions, alternatives, non-obvious hardware findings, and verification evidence. Keep each distinct milestone in its own digest and retain granular entries for unfinished work. The completion commit includes its own entry describing the outcome and compaction; do not add a second entry solely to satisfy the compaction procedure.

The two responsibilities are separate: **logging happens on every commit; compaction happens at milestone completion**. See [git-commits.md](git-commits.md) for staging and validation.

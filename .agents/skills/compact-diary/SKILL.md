---
name: compact-diary
description: Compact settled per-commit diary entries into separate milestone digests when a roadmap point or milestone completes.
---

# Diary Compaction at Milestone Completion

## Logging and Compaction Are Separate

Every commit adds a concise entry to [DIARY.md](../../../DIARY.md) Section 10 using `tools/harness/log_diary.py`, per [diary-maintenance.md](../../rules/diary-maintenance.md). The appender formats entries; it does not compact them.

Run this skill after a minor roadmap point or major milestone passes its completion gates, regardless of diary size. Consolidate its settled commit entries before the completion commit. A user may also explicitly request historical compaction.

## Preserve the Engineering Record

- Give each distinct milestone its own digest. Do not merge unrelated hardware, tooling, documentation, or workflow milestones into one generic entry.
- Preserve the problem, decisions, alternatives, non-obvious findings, affected components, and actual verification evidence. Distinguish historical results from checks run now.
- Omit transient debugging noise and mechanical micro-diffs already recorded in Git.
- Keep granular entries for unfinished work. Do not compact active investigation merely because the file is large.

## Procedure

1. Record the diary's line count and byte size. Read the relevant Section 10 range to establish which entries belong to the completed milestone and which remain active.
2. Draft a separate digest for each settled milestone, preserving the date range and context:

   ```markdown
   ### [YYYY-MM-DD HH:MM CET/CEST] — Milestone Digest: <Theme>
   - **Timestamp & Context**: <date range and milestone>
   - **Affected Subsystems**: <components>
   - **What Was Changed**: <established behavior or mechanisms>
   - **Architectural Rationale & Trade-Offs**: <decisions and alternatives>
   - **Verification & Invariants**: <observed results and limitations>
   ```

3. Replace only the settled entries with chronologically ordered digests. Review the diff for lost decisions, hardware findings, or evidence.
4. Run the per-commit checks from [git-commits.md](../../rules/git-commits.md), plus the applicable milestone gates.
5. Append the completion commit's own entry describing the result and compaction. Stage it with the milestone changes. Do not create a second diary-only commit or duplicate compaction entry.

## Report

Report the milestones compacted, active entries retained, before/after line and byte counts, important decisions preserved, and checks actually run. Label token savings as estimates if reported.

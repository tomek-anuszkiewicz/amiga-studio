# Engineering Diary

- Every commit includes a new entry in [DIARY.md](../../DIARY.md)'s `Rolling Changelog`, staged with that change, including documentation, rules, tests, and merges.
- Keep `## Rolling Changelog` as the final, unnumbered section. Append entries there regardless of how many numbered chapters precede it; do not refer to it by a section number.
- Record affected components, changes, rationale, and actual verification. Identify failures and checks not run; a commit alone does not prove milestone completion.
- Use the diary appender and timestamp format documented in [agent maintenance](../references/agent-tooling.md#recording-and-validating-changes), rather than reading the whole diary.
- After a minor roadmap point or major milestone passes its gates, use [compact-diary](../skills/compact-diary/SKILL.md), regardless of diary size.
- During compaction, review diary and Git/roadmap history for completed stages that merit a full narrative chapter. Add each distinct stage before the changelog using the next chapter number, or extend its existing chapter. Preserve dates, stable task IDs, decisions, alternatives, hardware findings, actual evidence and remaining limits. Move its settled entries into that chapter without duplicating the detailed account in the changelog.
- Treat the changelog as a buffer for work awaiting resolution or integration into the narrative. During compaction, move durable decisions and evidence into their owning chapters, including SDLC and development-tool history. Leave routine mechanical changes and compaction statistics in Git; do not preserve a separate digest merely because an entry existed. Use a dated digest only for substantive settled history with no suitable chapter, and reconsider its placement at the next compaction.
- Retain granular entries needed to continue unfinished work; mentioning an open roadmap stage alone is not a reason to retain a settled entry. Roadmap removal or renumbering alone does not prove completion: distinguish verified closure, user-directed scope closure and pending work. The completion commit gets one entry covering completion and compaction.

Commit validation belongs to [git-commits.md](git-commits.md); milestone obligations belong to [roadmap-maintenance.md](roadmap-maintenance.md).

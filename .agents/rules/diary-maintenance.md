# Engineering diary

Every ordinary or final integration commit includes a prepared entry in
[DIARY.md](../../DIARY.md)'s final, unnumbered `## Rolling Changelog`, staged with its
change. The narrow [integration working/assembly-commit exceptions](git-merge-commits.md#authorization-and-preparation)
record their unverified status in the final closure entry instead.

Prepare all entries before freeze. Record components, change, rationale, observed
checks, failures and unrun work; the final profile outcome belongs in commit messages
and the completion report. No diary edits between frozen commits or during publication.
Appending mechanics belong to [TA-prepare-batch](../skills/TA-prepare-batch/SKILL.md)
and the [diary appender reference](../references/agent-tooling.md#recording-and-validating-changes).

Minor-roadmap/major-milestone closure requires compaction before final verification
and freeze, regardless of size. It is a separately selected
[TA-compact-diary](../skills/TA-compact-diary/SKILL.md) operation; ordinary appending
is not compaction. Preserve unfinished work and unique engineering history under
[history criteria](../references/diary-history.md). A completion commit has one entry
covering completion and compaction; historical results are not new evidence.

[Commit policy](git-commits.md) owns readiness and timing;
[roadmap policy](roadmap-maintenance.md) owns closure obligations.

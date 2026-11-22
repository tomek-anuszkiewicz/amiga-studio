---
name: TA-WF-Step-compact-diary
description: Consolidate selected completed work into thematic diary chapters while preserving historical evidence and its limits.
---

# Compact the selected settled history

Apply [the result contract](../../references/workflow-contract.md).
Apply the history/document ownership parts of the shared
[information hierarchy](../../references/information-hierarchy.md).
The main session compacts the selected part of the diary when the user explicitly
requests it or the closure trigger is met. Compaction is part of
[TA-WF-Main-close-milestone](../TA-WF-Main-close-milestone/SKILL.md) and can also be
selected separately. Read the [closure trigger](../../../Obsidian/Amiga/Design/Verification/Testing%20Strategy%20and%20Quality%20Assurance.md#roadmap-and-milestone-completion).

1. Record current line/byte counts. Read affected changelog entries, owning chapters
   and relevant Git/closure evidence. Pruning alone does not establish completion.
2. Move substantive decisions, mechanisms and evidence from settled work into
   the chapters that cover those topics. Add a numbered chapter before the
   changelog when justified. Preserve historical evidence and its limits, and
   distinguish earlier checks from checks run now. Keep instructions for
   continuing a task in its plan.
3. Remove each source entry only after preserving information unique to it.
   Leave routine change details in Git. Do not create a second archive in prose
   or infer reasons the evidence does not establish.
4. Review the diff and incoming heading links. Keep the final unnumbered
   `Rolling Changelog`, then run
   `python .agents/skills/TA-WF-Step-compact-diary/scripts/check_diary.py`.
5. Report the chapters you changed, the evidence you preserved, the counts and
   the results of the checks you ran.

Stop after compaction. Ordinary commits do not require changelog entries. Do not
create a commit, run a milestone suite or claim historical closure from structure.

## History criteria

Apply these criteria when authoring an entry as well as when compacting history.
Record performed work, its reason, outcome and actual checks. For code changes,
use the selected committed scope as the source of the historical account. Link
relevant commits where useful. Completed investigations may retain their observed
results and limitations without claiming an implementation or verified closure.
Keep pending edits, commit status and instructions for continuing a task in its
plan; do not use the diary as a list of working-tree files awaiting commit.

Keep distinct completed engineering stages identifiable. Extend their existing
chapter or add the next numbered chapter immediately before the final, unnumbered
`## Rolling Changelog`. Consolidate related SDLC, harness and environment history
into a coherent account rather than one digest per maintenance change.

Preserve date ranges, stable task IDs, closing commits where available, problems,
affected components, mechanisms, decisions, alternatives, hardware findings,
historical verification and remaining limits. Read Git and roadmap history for
evidence of completion. A removed or renumbered roadmap item alone cannot
establish it. Say what verification confirmed, what scope the user chose to
close and what limits remain. Report acceptance of artifacts the tooling can
produce only when evidence for that claim goes beyond delivery of the tooling.

Retain concrete observations from completed work even when a wider roadmap stage
is still open. Continuing that work belongs to the active plan and roadmap.
The changelog holds completed work until its account can be incorporated into
the relevant chapter. Move any unique knowledge from settled work into that
chapter and remove the source entry to avoid repeating the detailed account. An empty changelog after
historical compaction is valid.

Leave transient debug noise, mechanical micro-diffs, routine documentation/merge
details and compaction statistics in Git. Extract unique knowledge before deleting
an entry; its prior existence does not establish permanent value. Use a dated
Historical Digest only for substantive settled history without a suitable chapter
and reconsider that placement next time. Reserve Milestone Digest for documented
milestone closure. Preserve useful incoming links when headings move.

Report the chapters changed, the important decisions and evidence preserved,
the changes in line and byte counts, and the checks actually run. The structure
and chronology checks establish format and timestamp order. Claims of closure
need their own evidence. Report pending gates as pending and attribute
historical results to the runs that produced them.

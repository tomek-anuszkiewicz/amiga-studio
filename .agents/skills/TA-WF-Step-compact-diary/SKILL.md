---
name: TA-WF-Step-compact-diary
description: Consolidate selected settled diary history into thematic chapters while preserving unfinished work and evidence.
---

# Compact the selected settled history

Apply [the result contract](../../references/workflow-contract.md).
Apply the history/document ownership parts of the shared
[information hierarchy](../../references/information-hierarchy.md).
Executor: main session.
Inputs: diary scope and explicit compaction request or actual closure trigger.
This operation is a component of
[TA-WF-Main-close-milestone](../TA-WF-Main-close-milestone/SKILL.md); it also remains
available for separately selected compaction.
Read the [closure trigger](../../../Obsidian/Amiga/Design/Verification/Testing%20Strategy%20and%20Quality%20Assurance.md#roadmap-and-milestone-completion).

1. Record current line/byte counts. Read affected changelog entries, owning chapters
   and relevant Git/closure evidence. Pruning alone does not establish completion.
2. Integrate substantive settled decisions/mechanisms/evidence into their owning
   thematic chapters; add a numbered chapter before the changelog when justified.
   Preserve granular unfinished work and distinguish historical from current checks.
3. Remove source entries only after retaining unique knowledge. Routine change noise
   stays in Git; do not create a parallel prose archive or infer missing rationale.
4. Review the diff and incoming heading links. Keep the final unnumbered
   `Rolling Changelog`, then run
   `python .agents/skills/TA-WF-Step-compact-diary/scripts/check_diary.py`.
5. Return chapters changed, preserved active work, counts and actual check results.

Stop after compaction. Ordinary commits do not require changelog entries. Do not
create a commit, run a milestone suite or claim historical closure from structure.

## History criteria

Keep distinct completed engineering stages identifiable. Extend their existing
chapter or add the next numbered chapter immediately before the final, unnumbered
`## Rolling Changelog`. Consolidate related SDLC, harness and environment history
into a coherent account rather than one digest per maintenance change.

Preserve date ranges, stable task IDs, closing commits where available, problems,
affected components, mechanisms, decisions, alternatives, hardware findings,
historical verification and remaining limits. Inspect Git and roadmap history;
roadmap removal or renumbering alone does not prove verified completion. Distinguish
verified closure, user-directed scope closure and unfinished work. Delivered tooling
does not establish acceptance of every artifact it can produce.

Keep granular entries needed to continue active investigations or unfinished work.
An open roadmap stage does not make every supporting tooling change unfinished.
The changelog is a buffer awaiting resolution or narrative integration, not a
parallel archive. Move settled unique knowledge into its owning chapter and remove
the source entry without repeating the detailed account. An empty changelog after
historical compaction is valid.

Leave transient debug noise, mechanical micro-diffs, routine documentation/merge
details and compaction statistics in Git. Extract unique knowledge before deleting
an entry; its prior existence does not establish permanent value. Use a dated
Historical Digest only for substantive settled history without a suitable chapter
and reconsider that placement next time. Reserve Milestone Digest for documented
milestone closure. Preserve useful incoming links when headings move.

Report the chapters changed, important decisions preserved, active entries retained,
line/byte changes and actual structure/chronology checks. Those checks establish
format and timestamp order, not the truth of closure claims. Pending gates remain
pending; historical results are not checks executed now.

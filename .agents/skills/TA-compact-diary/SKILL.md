---
name: TA-compact-diary
description: Consolidate selected settled diary history into thematic chapters while preserving unfinished work and evidence.
---

# Compact the selected settled history

Apply [the invocation contract](../../references/workflow-contract.md).
Executor: main session; result kind: `documents`.
Inputs: diary scope and explicit compaction request or actual closure trigger.
Read [diary policy](../../rules/diary-maintenance.md) and
[history criteria](../../references/diary-history.md).

1. Record current line/byte counts. Read affected changelog entries, owning chapters
   and relevant Git/closure evidence. Pruning alone does not establish completion.
2. Integrate substantive settled decisions/mechanisms/evidence into their owning
   thematic chapters; add a numbered chapter before the changelog when justified.
   Preserve granular unfinished work and distinguish historical from current checks.
3. Remove source entries only after retaining unique knowledge. Routine change noise
   stays in Git; do not create a parallel prose archive or infer missing rationale.
4. Review the diff and incoming heading links. Keep the final unnumbered
   `Rolling Changelog`, then run
   `python .agents/skills/TA-compact-diary/scripts/check_diary.py`.
5. Return chapters changed, preserved active work, counts and actual check results.

Stop after compaction. Ordinary entry appending belongs to preparation. Do not
create a commit, run a milestone suite or claim historical closure from structure.


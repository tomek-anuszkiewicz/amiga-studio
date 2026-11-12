---
name: compact-diary
description: Compact settled diary history into dated digests and full chapters for completed stages, at roadmap completion or on explicit request.
---

# Diary Compaction and Completed-Stage Chapters

## Logging and Compaction Are Separate

Every commit adds a concise entry to [DIARY.md](../../../DIARY.md)'s final, unnumbered `Rolling Changelog` using `tools/log_diary.py`, per [diary-maintenance.md](../../rules/diary-maintenance.md). The appender formats entries; it does not compact them. Locate the changelog by its heading, never by a fixed section number.

Run this skill after a minor roadmap point or major milestone passes its completion gates, regardless of diary size. Consolidate its settled commit entries before the completion commit. A user may also explicitly request historical compaction.

## Preserve the Engineering Record

- Give each distinct stage its own chapter or digest. Do not merge unrelated hardware, tooling, documentation, or workflow milestones into one generic entry.
- Preserve the problem, decisions, alternatives, non-obvious findings, affected components, and actual verification evidence. Distinguish historical results from checks run now.
- Omit transient debugging noise and mechanical micro-diffs already recorded in Git.
- Keep granular entries for unfinished work. Do not compact active investigation merely because the file is large.

## Procedure

1. Record the diary's line count and byte size. Read the relevant changelog entries and existing chapters. Compare them with Git history, especially roadmap removals and closing commits, to separate settled stages from active work. Historical roadmap numbers can change; retain stable task IDs and closing commit references where available. Do not infer closure from pruning alone or turn old results into current verification.
2. Decide which settled stages merit full numbered chapters: a coherent completed capability, an architectural transition, or a closed roadmap stage with enough decisions and evidence to explain its outcome. Extend an existing chapter when it already owns that history; otherwise add the next numbered chapter immediately before `## Rolling Changelog`. Describe the outcome, date range, affected components, mechanism, rejected alternatives, historical verification and deferred scope in connected prose. Distinguish user-directed scope closure from verified completion; delivered tooling does not establish acceptance of every artifact it can produce.
3. For settled work that does not merit a chapter, draft a separate dated digest preserving its date range and context. Use `Historical Digest` for ordinary settled changes; reserve `Milestone Digest` for documented milestone closure:

   ```markdown
   ### [YYYY-MM-DD HH:MM CET/CEST] — Milestone Digest: <Theme>
   - **Timestamp & Context**: <date range and milestone>
   - **Affected Subsystems**: <components>
   - **What Was Changed**: <established behavior or mechanisms>
   - **Architectural Rationale & Trade-Offs**: <decisions and alternatives>
   - **Verification & Invariants**: <observed results and limitations>
   ```

4. Replace only settled entries with their chapters or chronologically ordered digests. Avoid repeating a chapter's detailed account in the changelog; use a short cross-reference only when needed. Keep unfinished work granular. Review the diff for lost decisions, hardware findings, evidence and incoming links to changed headings. Keep the changelog last and unnumbered.
5. Run focused checks for historical compaction. If a commit or milestone closure is authorized, follow [git-commits.md](../../rules/git-commits.md) and the applicable milestone gates. Compaction alone does not authorize staging or committing. The authorized commit gets its own changelog entry describing the result and compaction, staged with the related changes; do not create a second diary-only commit or duplicate compaction entry.
6. After compaction and any new entry, validate the final diary structure and
   timestamp order. This checks changelog ordering, not the truth of historical closure claims:
   ```powershell
   python .agents/skills/compact-diary/scripts/check_diary.py
   ```

## Report

Report the chapters added or extended, digests compacted, active entries retained, before/after line and byte counts, important decisions preserved, and checks actually run. Label token savings as estimates if reported.

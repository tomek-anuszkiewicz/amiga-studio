---
name: TA-WF-Main-close-milestone
description: Run the workspace public function scan and full SingleStepTests, remove completed roadmap work, compact its diary history, and report unusual word usage.
---

# Close the selected verified roadmap milestone

Apply the [result contract](../../references/workflow-contract.md).
Inputs: selected checkout, exact completed heading/item in
[ROADMAP.md](../../../ROADMAP.md) and its completion scope. Do not choose another
milestone or remove an unfinished requirement implicitly.

1. Read the selected roadmap section and the supplied information about its
   completion. Identify which items nested under that section are complete and
   preserve any distinct unfinished work. Ask the user to resolve an ambiguous
   item or unclear completion scope.
2. Confirm the required local tools/fixtures and avoid concurrent writers to
   shared outputs. Run `python tools/verification/verify.py milestone`, adding
   `--results-dir <path>` when selected, under the existing
   [verification command contract](../../../tools/verification/README.md#completion-verification).
   Verify first scans every Cargo workspace package for candidate unused public
   functions. It runs full SingleStepTests after the scan passes. Wait for Verify
   to exit, check its status and inspect the reports. If either check fails, is
   interrupted or cannot run, leave the roadmap and diary in place and report
   what happened. For each candidate, report the package, function, source
   location and report path. Tell the engineer to check callers and re-exports
   and remove confirmed unused code. For a failed probe, report the reason and
   any available log. The engineer directs repairs and commits. Rerun the full
   milestone profile after repairs before continuing with closure.
3. After both checks pass, remove the selected completed section under the
   criteria below, update affected numbering and references, and run
   `python tools/verification/checks/check_roadmap.py`.
4. Once verification succeeds and the selected scope is complete, consolidate
   the milestone's settled history using
   [TA-WF-Step-compact-diary](../TA-WF-Step-compact-diary/SKILL.md). Selecting
   milestone closure also authorizes this step. Preserve unrelated and unfinished
   entries, unique decisions and their evidence. Report if you find no
   applicable history. If compaction or its checks fail, report partial closure.
5. Apply [TA-WF-Step-report-word-usage](../TA-WF-Step-report-word-usage/SKILL.md) after the roadmap and
   diary edits. Show the 20 highest-ranked words, assess their use in context and
   suggest additions to the report's static vocabulary exclusions. This step
   reports possible wording problems; it does not rewrite documentation, update
   word lists or introduce a word-frequency pass/fail threshold. If the report
   cannot be completed, identify the unfinished reporting and preserve the actual
   test, roadmap and diary outcomes.
6. Return the removed scope, retained work, verification and roadmap-check results,
   diary changes/structure checks, word-usage assessment, proposed exclusions,
   newly discovered documentation directories and remaining issues.

Stop after verification, the roadmap edit, diary compaction and the word-usage
report. Do not add code review, implementation, commits, merge, push or worktree
cleanup to this stage.

## Roadmap removal criteria

Remove only the exact verified completed scope. Routine documentation synchronization
does not perform completion-based pruning. A source edit or prepared artifact alone
is not a passing milestone test. Failed or unavailable tests leave the selected item
in place.

Remove verified completed scope completely, without completed tags, checkboxes,
strikethrough or historical capability summaries. Preserve distinct unfinished
requirements with concrete gates, priorities and dependencies. Update numbering,
references and assigned checkpoints. Record removed/retained scope and its
supporting evidence. Do not claim stage closure until its selected completion work
is finished. Report actual checks and unfinished scope.

The roadmap check verifies that the root roadmap exists and contains no completed
`[x]` task checkboxes. It leaves numbering, links, completion evidence and milestone
readiness outside its checks. Inspect affected numbering and references separately.

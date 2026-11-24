---
name: TA-WF-Step-optimize-english-vocabulary
description: Assess repository English vocabulary and apply validated removals in a corrective scan or standalone cleanup. Support report-only assessment.
---

# Optimize the English vocabulary

Apply the [result contract](../../references/workflow-contract.md) and the
[validated removal rule](../../references/workflow-contract.md#validated-vocabulary-removals).
The main session uses this procedure during a due corrective English scan
or when the engineer selects vocabulary optimization. In standalone work,
start with the full report and contextual assessment from
[TA-WF-Step-correct-english](../TA-WF-Step-correct-english/SKILL.md).
Report-only work returns contextual findings, candidate entries and evidence
limits before any edits. For corrective work, use the procedure below within
the same language operation.

## Validate and apply removals

Use the current full report after any text or vocabulary edits. A report
with no removal candidates needs no validation. For each candidate, check its
source, owner, remaining uses and overlapping allowances. Prefer a sufficient
exact-file allowance for confined uses and an appropriately categorized
global declaration when uses outside those owners require it. Explain why
uncertain entries are retained or deferred.

1. Write a disposable JSON array under `.agent/tmp/` containing the exact `entry`
   objects you propose to remove. Run the read-only check on the entire batch.

   ```text
   python tools/verification/verify.py check english removals <proposal.json>
   ```

   Require a complete repository report with `validation.accepted` set to
   `true` and `validation.entries` matching the proposal. Exit one can accompany
   acceptance when the same unresolved occurrences survive. If entries mask one
   another, use the affected occurrences to narrow the batch. Retain the narrower
   sufficient allowance and prefer configured source order among equally suitable
   declarations. Keep groups whose support you cannot establish. Validate the
   resulting batch together. Wait for complete evidence before applying any batch.
2. Immediately before editing, compare current discovery and input bytes with the
   validated report. Use the scanner's repository discovery rules and the report's
   input digests. Revalidate after a change, including a newly eligible file or
   an edit to an approved declaration. Apply just the accepted entries to the
   current bytes. For word lists, retain inline comments on their own lines and
   preserve the indentation and line endings of surviving text. For exact-file
   allowances, remove the specified array elements and only the punctuation
   needed to keep the JSON valid. Preserve other entries, formatting and the Git
   index. Check each destination immediately before replacing it. If an edit is
   interrupted or an input changes during application, record the actual edits
   and rebuild the evidence from the current checkout before continuing.
3. Run one full check on the edited repository and compare the unknown words
   and source occurrences with the validated outcome, accounting for line
   shifts. Report removed, retained and deferred entries and unresolved
   findings. Investigate any difference and report actual edits with cleanup
   unconfirmed. This comparison ends the batch within the language operation.
   Leave the edits unstaged.

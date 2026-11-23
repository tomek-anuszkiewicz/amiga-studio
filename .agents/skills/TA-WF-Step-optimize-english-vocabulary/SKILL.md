---
name: TA-WF-Step-optimize-english-vocabulary
description: Assess the repository English vocabulary and remove only a validated batch of obsolete declarations when the engineer selects cleanup.
---

# Optimize the English vocabulary

Apply the [result contract](../../references/workflow-contract.md) and the
[validated removal rule](../../references/workflow-contract.md#validated-vocabulary-removals).
The main session carries out this operation in the selected checkout. A request
to optimize vocabulary selects a full repository assessment and justified edits.
A report-only request selects the assessment and leaves source files untouched.

1. Run `python tools/verification/verify.py check english` and open the report
   named by this invocation, including on exit zero. Use the
   [report contract](../../../tools/verification/README.md#english-vocabulary) to
   assess all findings and deferred owners. An absent or invalid report is a scan
   error. For report-only work, describe which entries are candidates, why they
   qualify and what evidence is missing. Return the assessment before any edits.
2. In cleanup, use the contextual rules in
   [TA-WF-Step-correct-english](../TA-WF-Step-correct-english/SKILL.md) in this
   assignment. Resolve clear unknowns first and preserve unresolved findings.
   Check each removal candidate against its source, owner, remaining uses and
   overlapping allowances. Prefer an existing exact-file allowance if it covers
   all genuine uses. Keep one appropriately categorized global declaration when
   uses outside those owners require it. Record the reason for retaining an
   uncertain entry. Use the shared rule for a pure addition. When making removals,
   use current scanner evidence after any correction or addition.
3. Write a disposable JSON array under `.agent/tmp/` containing the exact `entry`
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
4. Immediately before editing, compare current discovery and input bytes with the
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
5. Run the full check on the edited repository and compare the unknowns with the
   validated outcome, allowing for lines shifted by the accepted edits. Report
   what was removed, what was kept, unresolved findings and any differences
   between simulation and the final scan. Leave the edits unstaged.

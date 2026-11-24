---
name: TA-WF-Step-correct-english
description: Scan the full eligible working tree, correct authorized English text, assess vocabulary in context and apply validated dictionary maintenance, or report findings only.
---

# Check English and maintain the vocabulary

Apply the [result contract](../../references/workflow-contract.md).

The main session uses this procedure for a requested or due English check.
A request to run `verify.py check all` includes this procedure as part of the
aggregate check. A named file can set the editing focus. Every workflow scan
uses the full eligible working tree, even when the task has no changed eligible text.
Direct [vocabulary additions](../../references/workflow-contract.md#vocabulary-only-maintenance)
can proceed without a scan.

## Collect and read the full report

For `verify.py check all`, use the full report already produced by its English
step, including when a read-only runner supplies it. Keep the other checks'
results and failure handling. Refresh an affected check if an edit invalidates
its inputs.

For English checks outside that aggregate command, run the existing scanner
in the selected checkout.

```text
python tools/verification/verify.py check english
```

The scanner reads the current contents of eligible tracked and nonignored
new files and applies its exclusions, including `.agent/tasks/`.

Open the JSON report named by the current invocation's `English scan report`
line, including on exit zero. Read unknown occurrences, removal candidates,
deferred owners, completeness and errors. Attribute observations to that
invocation. A missing or invalid report or scanner error leaves the language
assessment incomplete. [English scan report contract](../../../tools/verification/README.md#english-vocabulary)

## Assess findings in context

Read each `unknown_word` in its surrounding sentence, identifier or example.
Check unfamiliar technical terms against local definitions and API usage or
authoritative sources before deciding whether to correct them. Use the
[prose criteria](../../references/narrative-criteria.md) and preserve meaning
and document structure. For an explicit report-only request, return contextual
findings and candidate removals with locations and evidence limits, then stop.
Leave the working files and Git index untouched.

For corrective work, edit authorized text and preserve other authors' working
files and staged content, including in shared dictionary files. Report findings
outside the authorized edit scope.

- Correct clear spelling, non-English or awkward prose while preserving meaning.
  Avoid unrelated rewrites. Ordinary comments and prose remain editable.
- Preserve valid API names, register/bit names, quotations, test literals and
  persisted keys whose exact spelling matters. Do not rename a valid identifier
  merely to satisfy the dictionary. A local spelling correction to an internal
  identifier may update its callers only when behavior remains unchanged.
- Add verified reusable terms to the appropriate configured list under
  `tools/verification/checks/english_words/`: programming, hardware, software or
  names. Include any required inflections explicitly. Keep valid technical wording
  instead of replacing it with an unnatural dictionary-friendly phrase.
- Use the scanner configuration's exact-file `file_words` entry for a necessary
  literal or deliberately invalid example confined to that file. Correct typos
  in ordinary prose. Assess each finding in context before deciding whether an
  exception is warranted, even when the scanner would accept a bulk addition.

Resolve clear cases without asking the programmer. Report only unresolved meaning,
unverifiable terms or choices that would alter behavior or an external contract.
For each, give the file, line, context, evidence and concrete options. Report read
or scanner errors separately from vocabulary findings.

## Finish the language operation

Repeat the full scan after correcting authored text. When vocabulary additions
changed scanner inputs and removals are under consideration, obtain a current
full report before validating them. When the only work is a justified addition,
retain the original result and apply the shared maintenance rule without
another scan, checks or review.

If the current report contains removal candidates, use
[Validate and apply removals](../TA-WF-Step-optimize-english-vocabulary/SKILL.md#validate-and-apply-removals)
in this operation. Report additions, removals, retained or deferred candidates
and unresolved findings, including occurrences in unrelated files. An
unresolved word or a scan error leaves the language result incomplete. Describe
partial application or a post-edit mismatch using the actual edits and results.
Leave the edits unstaged. Changes to scanner logic, discovery, exclusions or
other policies follow [protected verification](../../references/protected-verification.md#protected-verification).

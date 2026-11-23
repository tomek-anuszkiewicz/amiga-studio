---
name: TA-WF-Step-correct-english
description: Scan explicitly selected repository text, correct English, assess unknown vocabulary in context, and add justified vocabulary or exact-file exceptions without asking about clear cases.
---

# Correct selected English text and vocabulary

Apply the [result contract](../../references/workflow-contract.md).

Use this skill only when the user requests an English check. The user selects
the files or expressly requests a full repository audit. Requests to write or
implement something follow their own procedures. Ordinary
[vocabulary-only maintenance](../../references/workflow-contract.md#vocabulary-only-maintenance)
can proceed without this skill or its scanner. If the user asks for a report
only, report the findings. Otherwise, make contextual text corrections and add
justified vocabulary entries.

Run the existing scanner for the selected scope:

```text
python tools/verification/verify.py check english files <paths>
```

For an explicitly requested full audit, omit `files <paths>`. The scanner and
`tools/verification/checks/check_english.json` own discovery, exclusions and
vocabulary formats. Do not expand the requested scope to unrelated text.
Use the language-related [prose criteria](../../references/narrative-criteria.md)
for these corrections. Preserve meaning and document structure. Reorganizing an
article requires its own assignment.

Open the JSON report named in the `English scan report` line from this invocation,
including when the command returns zero. Read `findings` and the completeness and
exit status fields. Treat an absent or invalid report as a scanner error and use
the current command log to diagnose it. Report `scan_error` findings and deferred
owners. Report `unused_entry_candidate` findings as information within the
selected operation. Deletion requires the separately selected removal procedure.
[English scan report contract](../../../tools/verification/README.md#english-vocabulary)

Read each `unknown_word` in its surrounding sentence, identifier or example.
The scanner found no accepted spelling for that word in this context. Check unfamiliar
technical terms against local definitions and API usage or authoritative sources
before deciding whether to correct them. A term written in uppercase or present
in the source still needs this evidence.

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

Repeat the same selected scan after correcting authored text. When you only add
vocabulary, apply the shared maintenance rule and continue without another
scan, checks or review. Say what you added and why, and attribute the retained
scan results to the earlier run. Unresolved language findings, read errors or
scanner execution errors leave the assessment incomplete.

This operation does not stage, commit, publish, change scanner logic/scope or
launch another skill. Authority to edit justified exact-file vocabulary entries
does not authorize other scanner configuration changes.

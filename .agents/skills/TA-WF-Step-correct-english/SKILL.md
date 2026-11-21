---
name: TA-WF-Step-correct-english
description: Scan explicitly selected repository text, correct English, assess unknown vocabulary in context, and add justified vocabulary or exact-file exceptions without asking about clear cases.
---

# Correct selected English text and vocabulary

Apply the [result contract](../../references/workflow-contract.md).

Use only when the user requests this operation. The input is the selected files
or an expressly requested full repository audit. A request to write or implement
something does not invoke this skill. Respect a report-only assignment; otherwise
the operation permits contextual text corrections and justified vocabulary entries.

Run the existing scanner for the selected scope:

```text
python tools/verification/verify.py check english files <paths>
```

For an explicitly requested full audit, omit `files <paths>`. The scanner and
`tools/verification/checks/check_english.json` own discovery, exclusions and
vocabulary formats. Do not expand the requested scope to unrelated text.
Use the language-related [prose criteria](../../references/narrative-criteria.md)
within these corrections, preserving meaning and document structure; an article
reorganization requires its own assignment.

Read each finding in its surrounding sentence, identifier or example. The scanner
reports dictionary absence, not an established language error. Check unfamiliar
technical terms against local definitions and API usage or authoritative sources;
uppercase spelling or occurrence in source alone does not prove validity.

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
  literal or deliberate invalid example confined to that file. Do not add a typo
  in ordinary prose to an exception list. Neither bulk-copying findings nor the
  scanner's acceptance substitutes for contextual review.

Resolve clear cases without asking the programmer. Report only unresolved meaning,
unverifiable terms or choices that would alter behavior or an external contract.
For each, give the file, line, context, evidence and concrete options. Report read
or scanner errors separately from vocabulary findings.

After corrections, repeat the same selected scan and summarize text corrections,
vocabulary additions with reasons, remaining questions and the actual result.
Retain failed attempts. An unresolved finding or scanner/read error leaves the
language assessment incomplete; do not claim a passing audit or publication readiness.

This operation does not stage, commit, publish, change scanner logic/scope or
launch another skill. Authority to edit justified exact-file vocabulary entries
does not authorize other scanner configuration changes.

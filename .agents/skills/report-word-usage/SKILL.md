---
name: report-word-usage
description: Rank uncommon words in project-authored Markdown, assess vague wording in context, and suggest vocabulary exclusions. Use for a requested word-usage report or the report included in milestone completion.
---

# Report words that may hide an unclear explanation

Executor: main session. Apply the [result contract](../../references/workflow-contract.md)
and [communication rules](../../../AGENTS.md#communication).
This report helps the user notice repeated, vague language. It is an editorial
assessment, not a spelling check or a test of whether a milestone is complete.

From the selected checkout, run:

```text
python .agents/skills/report-word-usage/scripts/rank_words.py --output .agent/tmp/word-usage/report.json
```

The script reads current Markdown from the agreed project directories, including
tracked files and nonignored new files. Its fixed scope covers root project
documents, agent instructions, Design notes, reader documentation and the selected
tool, test, benchmark and example documentation. It excludes reference books,
`ref_src/`, `tools/bootstrap/books/`, task archives, dependencies and generated
outputs. Use the report's paths and directory totals to describe the actual scan;
do not expand the scope to find more candidates.

The script also inventories Markdown file names elsewhere in the checkout,
skipping known ignored source collections and build/cache directories. It reports
unclassified directories and example paths without reading their contents or
adding them to the word counts. Include these notices in the response so the user
can decide whether each directory belongs in the scan or in the ignore rules.
The script does not change its own scope.

The bundled [common words](assets/common-words.txt) and
[project vocabulary](assets/project-words.txt) are static exclusions. Their
[sources and maintenance guidance](references/vocabulary.md) explain what they
contain. Reports need no downloads or installed language packages. Counts ignore
letter case and retain different word forms separately. Metadata, code examples,
paths and link destinations do not count as prose.

Read the report and inspect its source examples. For each of the top 20 words,
decide whether the examples show a useful term, a possible vocabulary exclusion,
or wording that hides who acts, what happens or which condition applies. Read the
surrounding passage when a short sample is insufficient. Explain uncertainty and
mixed meanings rather than assigning one verdict to every occurrence of a word.
Do not treat frequency or absence from the common-word list as proof of misuse
or as an English proficiency level.

Show the 20 highest-ranked words with their occurrence and document counts. If
fewer than 20 remain, show the available words and state the number. Keep the
ranking produced by the stored filters. Separately suggest ordinary or established
project terms that should be excluded in a future run, explaining each suggestion.
Do not quietly remove them or add a familiar abstract word to the exclusions
merely because it occurs often.

Give a short assessment of the strongest wording concerns, supported by source
examples and locations. Where useful, show how naming the actor, action, data or
result would improve an example. Preserve its requirements and evidence limits.
Finish the prose report with a compact summary of the scanned directories and
the excluded source collections, followed by any newly discovered directories
that need a scope decision.

Report source-reading failures or other missing inputs as limits; an incomplete
scan cannot establish a complete ranking. Word frequencies and editorial findings
have no pass/fail threshold. Stop after the report and suggested exclusions.
Do not rewrite source documents, rename code, update the filters or start another
operation. The caller decides how to use the findings.

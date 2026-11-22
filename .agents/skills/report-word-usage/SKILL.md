---
name: report-word-usage
description: Rank uncommon words in project-authored Markdown, assess vague wording in context, and suggest vocabulary exclusions. Use for a requested word-usage report or the report included in milestone completion.
---

# Report words that may hide an unclear explanation

The main session prepares this editorial report to help the user find recurring
vague wording. The report assesses how words are used in context. Spelling and
milestone completion have their own checks. Follow the
[result contract](../../references/workflow-contract.md) and
[communication rules](../../../AGENTS.md#communication).

From the selected checkout, run:

```text
python .agents/skills/report-word-usage/scripts/rank_words.py --output .agent/tmp/word-usage/report.json
```

The script reads the current contents of tracked Markdown and new Markdown
files that Git does not ignore. Its fixed scope covers root project
documents, agent instructions, Design notes, reader documentation and the
selected tool, test, benchmark and example documentation. It excludes reference books,
`ref_src/`, `tools/bootstrap/books/`, task archives, dependencies and generated
outputs. Use the paths and directory totals in the report to describe the
scan within that fixed scope.

In the rest of the checkout, the script lists Markdown file names and skips
known ignored source collections and build and cache directories. For each
unclassified directory, it reports example paths and leaves their contents
outside the scan and the word counts. Include these notices so the user can
decide whether each directory belongs in a future scan or in the ignore rules.

The report uses bundled lists of excluded words, so it needs no downloads
or installed language packages. It ignores letter case, counts different
word forms separately and skips metadata, code examples, paths and link
destinations when it counts prose. [Common words](assets/common-words.txt),
[project vocabulary](assets/project-words.txt),
[sources and maintenance guidance](references/vocabulary.md).

Read the report and inspect its source examples. For each of the top 20 words,
decide from its context whether it names something useful, is a candidate for
exclusion or obscures who acts, what happens or which condition applies. Read
the surrounding passage when the sample leaves the meaning unclear. Explain
any uncertainty or differences between uses of the same word. Frequency and
absence from the common-word list alone establish neither misuse nor an
English proficiency level.

Show the 20 highest-ranked words and give each word's occurrence and document
counts. If fewer than 20 words remain, show them all and state how many there
are. Keep the ranking from the stored filters. Separately suggest ordinary
English words or established project terms to exclude in a future run. Explain
each suggestion from its use in context. For familiar abstract words,
frequency alone gives insufficient reason to suggest an exclusion.

Give a short assessment of the most significant wording concerns, with
examples and their locations. Where useful, show how naming the actor, action,
data or result would clarify a passage while preserving its requirements and
the limits of its evidence. End the prose report with a short account of the
scanned directories and excluded source collections, followed by any
newly discovered directories that need a scope decision.

Report any files the script could not read or other missing inputs. An
incomplete scan cannot establish that the ranking is complete. Present word
frequencies and editorial findings without a pass/fail threshold. The assignment
ends with the report and suggested exclusions. Changes to source documents,
code names or filters and any follow-up operation require a separate decision
from the caller.

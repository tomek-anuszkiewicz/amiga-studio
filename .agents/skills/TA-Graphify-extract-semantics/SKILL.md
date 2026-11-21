---
name: TA-Graphify-extract-semantics
description: Extract and apply semantics for the selected project documents through the current host agent and existing graph tools.
---

# Extract the selected project semantics

Apply [the result contract](../../references/workflow-contract.md).
Executor: main session.
Inputs: explicit project semantic-extraction scope/trigger, source snapshot and
output permission. Reference graphs are ineligible.

1. Follow [the fixed prepare/extract/apply procedure](references/procedure.md).
   Read the extraction schema only for the selected pending sources.
2. Extract evidence-grounded fragments inline within the selected scope and any supplied limit.
   Do not recursively delegate or choose another public skill.
3. Apply through the existing tool, inspect actual acceptance and return coverage,
   cache/provenance evidence, generated artifacts and semantic freshness limits.

Stop after extraction/apply or a concrete blocker. Do not install a backend,
switch corpora, rewrite source documents or begin RAG indexing.

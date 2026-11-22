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
2. The main session performs this operation directly using only this public
   skill. Extract fragments from source evidence and stay within the selected
   scope and any supplied limit.
3. Apply the fragments with the existing tool and inspect what it accepted.
   Report which sources the operation covered, what the evidence says about
   cache use and fragment origins, which files the tool generated and any
   limits on how current the semantic data is.

Stop after extraction/apply or a concrete blocker. Do not install a backend,
switch corpora, rewrite source documents or begin RAG indexing.

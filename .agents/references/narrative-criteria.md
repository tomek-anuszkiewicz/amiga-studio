# Documentation prose and narrative criteria

## All selected documentation

Apply these criteria within the assigned writing or review scope. A reference to
them does not authorize rewriting unrelated text or expanding a link, metadata,
language, specification or architecture assessment into a general editorial audit.
Diary preparation and compaction retain their own criteria.

State the document's purpose and main useful conclusion early. Use concrete terms,
active sentences and enough context for the intended reader to follow the reasoning.
Explain mechanisms, constraints and consequences together; distinguish observed
evidence, approved requirements, inference and unresolved questions. Use headings,
lists, tables and diagrams when they make the content easier to understand, rather
than imposing a fixed template. Remove repetition and filler while preserving
substantive reasoning, evidence, caveats and useful links.

Match the structure to the genre. Specifications expose contracts, constraints
and their evidence; plans expose scope, decisions and acceptance criteria;
procedures expose inputs, actions and observable outcomes. Keep operational detail
with its owner under [document/instruction ownership](information-hierarchy.md).
For selected reorganizations, read the complete target and its relevant source
evidence, then compare before and after for retained knowledge and resolving links.

## Narrative articles

Apply to the selected methodology article, essay, retrospective or case study,
not to every specification or task plan. Lead with the central conclusion or
architectural choice, normally in the opening 20–50 lines. Adapt section boundaries
to the article instead of mechanically imposing an outline.

| Progression | Contribution |
| --- | --- |
| Core conclusion | The decisive result, architectural choice or useful mental model |
| Strategic and human constraints | The problem, stakes, bottlenecks and failed approaches that made the decision necessary |
| Architectural solution | Mechanisms addressing those constraints, with a diagram when useful |
| Execution realities | Evidence-supported hardware timing, host execution, memory or compiler constraints |
| Tactical workflow | Relevant procedures, evidence boundaries and operational requirements, linking their owners |
| Synthesis and references | Connections and useful sources without repeating the article |

Place new findings where they advance that argument; do not append important
conclusions at the bottom by default. A changed thesis belongs near the opening,
an essential methodology step with the solution, and an operational command with
the tactical procedure. Inspect trailing sections and appendices for misplaced
high-impact conclusions.

Preserve substantive mechanisms, evidence, caveats, formulas, timing tables,
diagrams and useful links when restructuring. Remove duplication; move detail to
linked owning references when authorized. Do not discard unique reasoning to meet
a size target. Apply [Documentation Value](documentation-criteria.md#documentation-value)
to choose retained knowledge, and distinguish established evidence from inference.
Compare the final diff for completeness and resolving links. Ordinary article work
does not authorize staging, commits or unrelated audits.

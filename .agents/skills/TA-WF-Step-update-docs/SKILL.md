---
name: TA-WF-Step-update-docs
description: Assess documentation needs, author or restructure selected documents, reconcile them with approved requirements and implementation, and organize pending roadmap work.
---

# Update the selected documentation

Apply [the result contract](../../references/workflow-contract.md).
Apply shared [document/instruction ownership](../../references/information-hierarchy.md)
to the selected documents. This operation can assess or update documentation
independently of an implementation assignment.
The main session uses the selected code changes, assigned documents or findings,
or both as applicable. It also needs the requested outcome, exact candidate,
approved requirements, implementation and evidence, and write scope. For pending
roadmap work, use the user's priorities and known dependencies. Return `blocked`
if the document target is missing or ambiguous.

1. Read [Documentation Value](../../references/documentation-criteria.md#documentation-value),
   [specification compliance guidance](../../references/spec-compliance.md) and supplied findings.
   Apply the shared [prose and narrative criteria](../../references/narrative-criteria.md)
   according to the selected document's genre.
   Apply the [history criteria](../TA-WF-Step-compact-diary/SKILL.md#history-criteria)
   when writing the diary. Describe work performed and actual results from
   the selected committed scope. Keep instructions for continuing a task and
   working-tree status in its plan. Reading the criteria alone gives no
   authority to compact the diary or add an entry.
   For descriptions of replaced APIs, schemas or formats, apply the
   [internal migration contract](../../references/internal-migrations.md).
   Reader-facing docs explain the project and its entry points. Keep operating
   procedures with their tools or skills and update their consumers when they move.
   For selected code changes, use Design `tracked_paths` and relevant links
   between source and documents to identify affected documentation. Assess
   whether changed behavior, contracts or valuable knowledge call for updates.
   Keep any edits within the assigned write scope. Report gaps in document
   ownership and needed updates outside that scope.
2. Author or reconcile only assigned documents and compatible enrichment. For an
   article or substantial restructuring, read the complete document and
   relevant evidence. Trace the argument, then place each important conclusion
   with the reasoning it supports, including conclusions previously buried near
   the end.
   Apply the shared [Mermaid diagram criteria](../../references/documentation-criteria.md#mermaid-diagrams)
   to new or edited Mermaid diagrams.
   Preserve source-backed hardware explanations, independent requirements, provenance and
   unique rationale. Do not document a contradiction as accepted behavior.
   Omit straightforward code narration unless it adds independent value. Missing
   entity descriptions alone are not defects, and specifications need not narrate
   every implementation detail. Preserve source-confirmed hardware facts even
   when they are easy to read from code. Documenting a feature does not authorize it.
3. For assigned pending roadmap organization, apply the criteria below.
   Consolidate and order pending work without declaring completion or removing
   completed items. Preserve unfinished scope and return any missing completion
   evidence for a separately selected milestone closure.
4. Whenever you edit a document with a metadata header, check every field
   against the revised content and its schema. For Design notes, apply the
   [vault schema](../../references/vault-metadata.md). Set `updated` to the
   date of a substantive content change. When correcting a stale date, use
   Git to identify the latest substantive change. Check that the title,
   aliases, tags, category, subsystem, status, related links and tracked paths
   still describe the note. Correct the fields that need it and preserve the
   original creation date. A review or mechanical edit alone leaves `updated`
   unchanged.
   Follow the separate [reference book schema](../../../tools/bootstrap/book-to-markdown/reference-book-metadata.md)
   and its approval requirement before editing source notes. Preserve approved
   intent, neighboring structure and links to implementation. Keep gaps as
   brief factual caveats linked to the owning [roadmap](../../../ROADMAP.md).
   Reuse shared specification owners and update General Architecture dependencies
   when crate relationships change.
5. Compare before/after content for retained unique reasoning, mechanisms,
   caveats, formulas, timing tables, diagrams and useful links, including content
   deliberately moved to an authoritative reference. Review the diff and relevant
   local links; after roadmap edits run
   `python tools/verification/checks/check_roadmap.py`.
   Report the documents you changed, addressed IDs, actual checks, conflicts
   and missing evidence. If evidence from the required final full vault checks
   is missing, report the checks as outstanding prerequisites.

Stop after document changes. Do not repair code/tests, perform an unrelated audit,
index documentation, begin integration, stage or commit.

## Organizing pending roadmap work

[ROADMAP.md](../../../ROADMAP.md) holds prioritized pending and active work. History
belongs in diary/Git. Organization does not infer completion or authorize milestone
closure.

Read the assigned sections, user priorities and known dependencies. Distinguish
pending requirements from descriptions, methods and history. Keep work pending
when its completion is uncertain. Group related objectives and remove duplicate
wording while retaining distinct requirements, acceptance evidence, failure
handling and open decisions. Describe near-term work concretely and keep
distant work concise. Store detailed execution plans under `.agent/tasks/`.

Order by user priorities and actual dependencies. For hardware work, account for
bus/clocks, DMA, display, peripherals and firmware dependencies; preparatory
conversion may precede implementation. Update affected numbering and references.
Organization alone does not authorize deleting items as completed.

The roadmap check verifies that the root roadmap exists and contains no completed
`[x]` task checkboxes. It does not validate numbering, links, completion evidence or
milestone readiness; inspect affected numbering and references separately.

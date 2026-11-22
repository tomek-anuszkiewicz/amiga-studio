---
name: TA-WF-Step-update-docs
description: Assess documentation needs, author or restructure selected documents, reconcile them with approved requirements and implementation, and organize pending roadmap work.
---

# Update the selected documentation

Apply [the result contract](../../references/workflow-contract.md).
Apply shared [document/instruction ownership](../../references/information-hierarchy.md)
to the selected documents. This operation can assess or update documentation
independently of an implementation assignment.
Executor: main session.
Inputs: selected code changes and/or assigned documents/findings, requested outcome,
exact candidate, approved requirements, implementation/evidence and write scope.
Pending roadmap work also needs user priorities and known dependencies. A missing
or ambiguous document target returns `blocked`.

1. Read [Documentation Value](../../references/documentation-criteria.md#documentation-value),
   [specification compliance guidance](../../references/spec-compliance.md) and supplied findings.
   Apply the shared [prose and narrative criteria](../../references/narrative-criteria.md)
   according to the selected document's genre.
   Diary authoring follows the [history criteria](../TA-WF-Step-compact-diary/SKILL.md#history-criteria):
   describe performed work and actual results from the selected committed scope;
   task continuation and working-tree status belong in plans. Reading these criteria
   does not select diary compaction or authorize a new entry on its own.
   For descriptions of replaced APIs, schemas or formats, apply the
   [internal migration contract](../../references/internal-migrations.md).
   Reader-facing docs explain the project and its entry points. Keep operating
   procedures with their tools or skills and update their consumers when they move.
   For selected code changes, identify affected documentation through Design
   `tracked_paths` and relevant source/document links. Assess whether changed
   behavior, contracts or valuable knowledge need an update; a code change does
   not by itself require more documentation. Keep edits within the assigned write
   scope and report missing ownership or out-of-scope document changes.
2. Author or reconcile only assigned documents and compatible enrichment. For an
   article or substantial restructuring, read the complete target and relevant
   evidence, map its argument and place important conclusions with the reasoning
   they support, including conclusions previously buried in trailing sections.
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
4. Apply selected metadata/link repairs using the
   [vault schema](../TA-audit-vault-links/references/vault-metadata.md).
   Follow the separate [reference book schema](../../../tools/bootstrap/book-to-markdown/reference-book-metadata.md)
   and its approval requirement before editing source notes. Preserve approved
   intent, neighboring structure and links to implementation. Update dates and
   relevant status, tags or dependencies for substantive changes. Keep gaps as
   brief factual caveats linked to the owning [roadmap](../../../ROADMAP.md).
   Reuse shared specification owners and update General Architecture dependencies
   when crate relationships change.
5. Compare before/after content for retained unique reasoning, mechanisms,
   caveats, formulas, timing tables, diagrams and useful links, including content
   deliberately moved to an authoritative reference. Review the diff and relevant
   local links; after roadmap edits run
   `python tools/verification/checks/check_roadmap.py`.
   Return changed documents, addressed IDs, actual checks, conflicts and missing
   evidence. Required final full vault
   checks remain explicit prerequisites when not already supplied.

Stop after document changes. Do not repair code/tests, perform an unrelated audit,
index documentation, begin integration, stage or commit.

## Organizing pending roadmap work

[ROADMAP.md](../../../ROADMAP.md) holds prioritized pending and active work. History
belongs in diary/Git. Organization does not infer completion or authorize milestone
closure.

Read the assigned sections, user priorities and known dependencies. Distinguish
pending requirements from descriptions, methods and history. Preserve uncertain
completion as pending. Consolidate related objectives and duplicate wording while
retaining distinct requirements, acceptance evidence, failure handling and open
decisions. Keep near-term work concrete and distant work concise; detailed
execution plans belong under `.agent/tasks/`.

Order by user priorities and actual dependencies. For hardware work, account for
bus/clocks, DMA, display, peripherals and firmware dependencies; preparatory
conversion may precede implementation. Update affected numbering and references.
Organization alone does not authorize deleting items as completed.

The roadmap check verifies that the root roadmap exists and contains no completed
`[x]` task checkboxes. It does not validate numbering, links, completion evidence or
milestone readiness; inspect affected numbering and references separately.

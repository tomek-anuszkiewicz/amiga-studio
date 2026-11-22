# Documentation and governance criteria

## Prose and narrative

Assess clarity, evidence placement and retained reasoning using the shared
[prose and narrative criteria](narrative-criteria.md), within the assigned document
scope. Use the article-specific progression for articles, not as a template for
specifications, plans or procedures. Link/metadata audits retain their narrower
scope; diary preparation and compaction have their own criteria.

For diary changes, apply the [history criteria](../skills/TA-WF-Step-compact-diary/SKILL.md#history-criteria).
Check that the account describes work that happened and reports evidence from
the selected committed changes. Keep pending edits, commit status and task
continuation in plans. Preserve historical verification limits without turning
them into a list of current tasks.

## Documentation Value

Preserve descriptions of hardware behavior confirmed by authoritative sources.
They define a contract independent of the implementation, even when the behavior
is easy to read from code. Also preserve knowledge that would take substantial
work to recover or explain, including established intent, decisions,
consequential trade-offs, external evidence and non-obvious constraints.

For other implementation descriptions, ask whether the reader can establish the
fact quickly and directly from code. If so, omit it unless it adds independent
value. Remove straightforward code narration, consolidate duplicates at their
owner, and retain valuable parts of mixed passages. Missing entity descriptions
alone are not defects. There is no size target or deletion quota.

Approved requirements retain intended behavior independently of current code.
Before deleting potentially unique knowledge, inspect relevant decisions, history,
tests or sources. Preserve minimum intent and evidence; distinguish established
rationale from inference and leave unknown reasons unknown.

### Hardware Explanations and Evidence

The owning specification explains hardware behavior with links to manual sections,
book passages or authoritative observations. Scoped review compares affected
behavior with those sources. Source-confirmed behavior missing its explanation
requires compatible enrichment at the owning specification, even when code makes
it obvious. Report-only review identifies that work and any resulting readiness
gap. The user selects any subsequent documentation update.

Hardware correctness requires evidence beyond code agreement. Distinguish what
sources establish about the hardware from choices made in the emulator model,
assumptions and questions the evidence leaves open. Report contrary
behavior, code, specification and source passages under
[specification compliance](spec-compliance.md); never remove a requirement to match
incorrect code. Preserve source books, imported references, conversion prompts and
generated artifacts. This editorial policy applies to project-authored explanations,
not rewriting their sources or executable instructions.

## Select the complete intended scope

Identify the baseline, selected paths and hunks, new files and proposed groups
of changes. Checks can still read files in the working checkout after you stage
selected changes. If the results depend on unrelated working files, they leave
the staged candidate's result unverified. For integrated work, identify
the delivered commit range and any selected working changes, including merge
resolutions. Review the complete intended result in the selected checkout.
Report anything that interferes with the assessment or leaves part of the
comparison undefined.
No frozen baseline copy or publication control record is required. Use
`--no-renames` when selecting paths to expose both endpoints.
For an initial commit inspect the intended candidate without assuming HEAD exists.

Collect `tracked_paths` from Design frontmatter. Normalize slash paths and match an
exact file or directory boundary: `crates/agnus` does not match `crates/agnus_extra`.
Select every intersecting specification, including non-crate paths, deleted/renamed
owners and explicitly changed Design documents. Report invalid mappings and
unmapped changed crate code; resolve ownership from relevant domain references,
never an invented association or a whole-vault audit to hide missing scope.
Workspace/dependency changes select affected manifests and General Architecture.

Changed requirements also select their affected implementation and tests when those
files have no diff. Rules/tooling changes without Design mappings select directly
affected instructions and consumers, not every hardware subsystem. Editorial
changes alone do not require implementation changes.

## Assess meaning and authority

Check that behavior and tests satisfy the intended requirements. Also check that
the intended prose retains source-supported behavior and valuable knowledge.
Inspect relevant registers, timing phases, ownership, public contracts and shared
interactions. Extend scope only for a concrete dependency, and record why. When
a requirement changes, inspect its previous wording and look for evidence that
the user authorized the change. Agreement between the final code and prose
establishes consistency between them. Authorization requires its own evidence.

Classify findings as unmet requirements, approved changes awaiting implementation,
compatible knowledge gaps, unsupported claims or genuine specification conflicts.
Do not legitimize divergence, invent historical rationale or remove independent
hardware requirements. Accurate documents and dates stay unchanged. Substantive
corrections update metadata; no synchronization hashes, periodic stamps or
review-only dates. Design specifications have no line-count ceiling.

At minor-roadmap/major-milestone closure assess the modified subsystems and their
interactions across the whole stage, including unchanged code. Static semantic
checks establish only their configured comparisons. A matching broader audit can
satisfy scoped compliance when snapshot, requirements and evidence overlap;
record that coverage instead of repeating it. Unresolved conflicts, valuable
knowledge gaps or required implementation/evidence gaps block readiness.

## Instructions and procedures

Use [instruction ownership](information-hierarchy.md). Each requirement,
criterion and procedure has one authoritative home with resolving consumer links.
Trace a realistic invocation of the affected procedure. Identify who authorizes
it and confirm that its rules come from the sources that own them. Check which
scope and inputs it uses, which evidence it requires, what happens on failure
and where it stops. Look for duplicate readers, contradictory transitions,
unreachable steps and examples that still direct readers to retired mechanisms.
Shortening the root instructions preserves selective reading only if the linked
references do not introduce a compulsory reading chain. Check runtime isolation
from actual behavior. Review labels and filenames cannot establish it.

For instruction/role/skill/verification changes, identify affected reviewer criteria
and update their routing together. Other lane assessment belongs to the caller;
this review never dispatches it. No cosmetic changes are required to unaffected
checklists. Assess script/function ownership by actual responsibilities, including
switches in mixed scripts; reference counts alone do not decide placement.

For selected Git/documentation changes, assess the intended combined result,
including conflict-free changed Markdown. Preserve unfinished plans and useful
outputs before an authorized worktree removal. No publication control record,
sealed candidate or fixed sequence of reviews is required.

Graph criteria include explicit project/reference roots, read-only queries,
worktree copies of existing graphs, project refresh within verified task closure,
separately selected reference refresh/production and
distinct structural/semantic freshness. Preserve the repository-owned skill on
installation. Relevant vault metadata/link semantics use the
[shared vault reference](../skills/TA-audit-vault-links/references/vault-metadata.md).
Check language, portable paths, resolving links and actually triggered diary,
roadmap and milestone obligations. Pending-work organization belongs to
[TA-WF-Step-update-docs](../skills/TA-WF-Step-update-docs/SKILL.md#organizing-pending-roadmap-work);
completion-based removal belongs to
[TA-WF-Main-close-milestone](../skills/TA-WF-Main-close-milestone/SKILL.md#roadmap-removal-criteria).
Assess their criteria without launching either operation. Retain unrelated and
historical material.

Report findings only. A recommendation to enrich, repair, index or verify is not
authorization to perform that operation. Shared criteria can be assessed within
one assigned documentation lane without launching several audit skills.

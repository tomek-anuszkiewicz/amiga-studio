# Documentation and governance criteria

## Prose and narrative

Assess clarity, evidence placement and retained reasoning using the shared
[prose and narrative criteria](narrative-criteria.md), within the assigned document
scope. Use the article-specific progression for articles, not as a template for
specifications, plans or procedures. Link/metadata audits retain their narrower
scope; diary preparation and compaction have their own criteria.

## Documentation Value

Preserve actual hardware behavior confirmed by authoritative sources, even when it
is easy to read from code: it remains an independent implementation contract.
Preserve knowledge whose discovery/explanation would otherwise require substantial
work: established intent, decisions, consequential trade-offs, external evidence and
non-obvious constraints.

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

Code agreement alone does not establish hardware correctness. Distinguish hardware
facts from modeling choices, assumptions and unresolved evidence. Report contrary
behavior, code, specification and source passages under
[specification compliance](spec-compliance.md); never remove a requirement to match
incorrect code. Preserve source books, imported references, conversion prompts and
generated artifacts. This editorial policy applies to project-authored explanations,
not rewriting their sources or executable instructions.

## Select the complete intended scope

Identify baseline, selected paths/hunks, new files and proposed partition. Staging
does not isolate execution, and unrelated working files cannot satisfy an index
candidate. For integration use the recorded immutable primary-branch base, including
committed task changes and merge resolutions; retain HEAD/index identities to
detect interference. Use `--no-renames` when selecting paths to expose both endpoints.
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

Compare intended versions in both directions: requirements against behavior and
tests; source-supported behavior and valuable knowledge against retained prose.
Inspect relevant registers, timing phases, ownership, public contracts and shared
interactions. Extend scope only for a concrete dependency and record why. Review
previous wording and approval evidence for changed requirements; final agreement
between code and prose does not establish authorization.

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
Walk a realistic affected invocation through authority, scope, inputs, required
evidence, failure handling and its stopping boundary. Find duplicate readers,
contradictory transitions, unreachable steps and active examples for retired
mechanisms. A shorter root with compulsory transitive reading is not selective
loading. Review labels and filenames do not establish runtime isolation.

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

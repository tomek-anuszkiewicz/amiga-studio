# Documentation and governance criteria

Apply [Documentation Value](../rules/docs-maintenance.md#documentation-value) to
additions and deletions: preserve approved requirements, source-backed hardware
explanations, provenance, established rationale and non-obvious constraints.
Missing descriptions of code entities alone are not defects. Existing code is
implementation evidence, not authority to rewrite a requirement.

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

Use [instruction ownership](../rules/information-hierarchy.md). Each requirement,
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

Integration criteria include approved immutable source/target selection, one
combined candidate, semantic reconciliation of every changed Markdown path
including conflict-free paths, task-specific closure records, and a cleanup
control record surviving source removal. Confirm publication cannot start repairs,
verification or new commits. Keep advanced multi-source acceptance at its rule/tool
owner; successive publication of separately tested sources is not combined review.

Graph criteria include explicit project/reference roots, read-only queries,
project-only preparation/worktree refresh, explicit reference production and
distinct structural/semantic freshness. Preserve the repository-owned skill on
installation. Relevant vault metadata/link semantics use the
[shared vault reference](../skills/TA-audit-vault-links/references/vault-metadata.md).
Check language, portable paths, resolving links and actually triggered diary,
roadmap and milestone obligations. Retain unrelated and historical material.

Report findings only. A recommendation to enrich, repair, index or verify is not
authorization to perform that operation. Shared criteria can be assessed within
one assigned documentation lane without launching several audit skills.

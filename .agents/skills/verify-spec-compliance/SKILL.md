---
name: verify-spec-compliance
description: Before every authorized Git commit, verify implementation against approved specifications using tracked_paths, enrich compatible implementation details, and block unresolved conflicts or verification gaps.
---

# Specification Compliance Review Before Commit

Follow [docs-maintenance.md](../../rules/docs-maintenance.md) and
[git-commits.md](../../rules/git-commits.md). Approved specifications govern this review;
it does not authorize staging, committing, or modifying unrelated work.

## When to Use

Mandatory before every authorized Git commit, including documentation, rules,
tests and merge commits. Review only the intended commit snapshot. Ordinary
implementation or refactoring alone does not trigger this procedure. Milestone
closure may use its broader semantic audit to satisfy this review when it covers
the same intended snapshot, selected requirements and evidence. Record that
coverage instead of repeating the comparison; recheck when relevant inputs change.
Explicit review requests remain valid.

## 1. Establish the Intended Snapshot

Identify the exact paths and changes agreed for this commit before reviewing.
Do not treat every dirty file as part of the commit or stage files merely to
select a documentation scope.

For an already staged snapshot, use:

```powershell
git diff --cached --name-only --no-renames -z
git diff --cached -- <selected_paths>
```

Using `--no-renames` exposes both old and new paths as deletion/addition. Include
both when selecting documents; compare old content from HEAD for deleted files.
For a commit not yet staged, use `git diff HEAD -- <intended_paths>` for tracked
files and read only explicitly selected untracked files. An untracked file is
not present in Git's diff. If the intended patch is partial or a file has both
staged and unstaged edits, inspect the actual proposed version and exclude
unrelated hunks. Do not use a worktree diff as evidence for a different index
snapshot. For an initial commit, use the intended index without assuming HEAD
exists. For a merge, inspect the intended result against its first parent and
review conflict resolutions and relevant changes from the other parent.

If the intended paths or patch cannot be identified, report the missing scope
and establish it before proceeding. Never substitute the whole working tree,
an arbitrary earlier commit, or the complete repository history.

## 2. Select Documents by Path Ownership

Read only YAML frontmatter from specifications under
`Obsidian/Amiga/Design/`, including its subdirectories, to collect their `tracked_paths`. Keep this mapping in
the documents; do not maintain a second crate-to-document table here.

Normalize repository-relative paths to forward slashes. A tracked path matches
an identical file path or a descendant under a directory boundary: `crates/agnus`
may match `crates/agnus/src/agnus.rs`, but not `crates/agnus_extra/...`.

Group intended code changes by `crates/<crate>/`, then select every specification
whose tracked paths intersect those changes. One crate may select several
documents; a file-specific mapping selects only changes to that file. Apply the
same matching to non-crate paths explicitly listed in `tracked_paths`.

- Check that configured paths resolve to existing files/directories, or are
  explained by an intended deletion or rename. Update mappings when ownership
  changes; do not silently skip an invalid path.
- Report changed crate code with no matching specification. Determine the
  appropriate document from existing domain references and resolve the missing
  mapping before claiming its documentation review complete. Do not invent an
  association or inspect every subsystem to compensate.
- Include design documents explicitly changed in the intended commit, even if
  they have no tracked paths. For each changed requirement, use its tracked
  paths and source links to locate the affected implementation and tests even
  when no code files changed. Resolve missing ownership from relevant domain
  references before claiming review complete. Editorial edits alone do not
  require implementation changes. For changes solely to rules or tooling with
  no mapping, review their directly affected instructions and links, not all
  emulator specifications.
- Workspace/dependency changes require the affected crate manifests and
  [General Architecture](../../../Obsidian/Amiga/Design/General%20Architecture.md),
  not every implementation file.

## 3. Review the Selected Changes

Read each selected document and inspect the intended changed files, their
diff and the implementation and tests affected by changed requirements,
including files with no diff. Use their versions in the intended snapshot;
unrelated working-tree edits cannot satisfy a staged requirement change.
Evaluate behavior, registers, timing phases,
bus coordination, public contracts and dependency changes described by the
specification. Compare in both directions within that scope: identify intended
code behavior missing from the selected specifications, and identify claims in
those specifications that the affected code does not support. This scoped review
does not replace the broader milestone semantic audit. Extend to another subsystem
only when a concrete shared contract, dependency or interaction requires it;
record the reason and added scope.

Distinguish the approved requirements, any user-approved requirement changes,
and compatible implementation details. When requirements changed, inspect the
previous wording and recorded user decision; matching final code and documents
alone does not establish that a changed contract was authorized.

Apply [spec-compliance.md](../../rules/spec-compliance.md) to classify each
difference as compatible enrichment, code failing a requirement, an approved
requirement change awaiting implementation, or a conflict/unauthorized extension.
Do not rewrite a specification simply to legitimize implementation divergence.

## 4. Resolve Differences Against Approved Requirements

For each changed requirement, identify the supporting implementation and
relevant test evidence. If implementation is missing or differs, update code
and tests within the authorized scope to satisfy the approved specification.
If the necessary work exceeds that scope, present the requirement, contrary
code behavior and needed scope, then obtain direction before expanding it.
Leave the review and commit blocked until the difference is resolved and
verified in the intended snapshot. Do not weaken or revert an approved
requirement merely to preserve existing behavior.

Enrich affected descriptions, register/timing tables and dependency diagrams
only with significant implementation details compatible with the approved
requirements and authorized scope. Follow
[compatible documentation enrichment](../../rules/spec-compliance.md#compatible-documentation-enrichment);
do not require a description of every helper or local algorithm. Missing detail
alone does not require approval. Conflicts and unauthorized extensions require
the recorded user decision specified by that rule; do not silently remove a
requirement, broaden the scope or describe divergent code as accepted behavior.

Prune superseded drafts and copied implementations only within the reviewed
scope; retain concise links to living source. Update `tracked_paths` if ownership
changed. Design specifications have no line-count ceiling.

Apply the class-scope and duplication rules in
[docs-maintenance.md](../../rules/docs-maintenance.md#living-design-specifications).
Review does not require a new paragraph for each changed instruction.

If no significant compatible detail is missing and the document describes the
approved behavior correctly, leave the file unchanged, including its `updated`
date. Change `updated` only alongside a real
content correction. Never add synchronization hashes/dates, periodic stamps or
review-only metadata. A successful review can produce no file edits and no
additional commit.

## 5. Validate and Return to the Commit Workflow

After document edits, follow
[obsidian-vault-linking](../obsidian-vault-linking/SKILL.md) for metadata and link
checks. Include only the necessary documentation corrections in the agreed
atomic commit together with required implementation and test changes. Recheck
selection if intended code, ownership or the proposed snapshot changes.
The enclosing workflow runs verification on the final source
and stages only authorized paths; do not repeat its gates inside this skill.
Do not return a passing review while an unresolved requirement conflict,
significant documentation gap or unverified changed requirement remains.
Passing mechanical checks does not replace this semantic
review, including for a commit containing only documentation changes.

## Report

Record the intended snapshot, changed crates/paths, selected documents, concrete
corrections or no-change findings, scope extensions, missing/invalid mappings,
approved requirements and decisions, their implementation/test evidence,
compatible documentation additions, unresolved differences or verification gaps,
and checks actually run. Do not claim unchanged
documents were updated or that an unreviewed subsystem was verified.

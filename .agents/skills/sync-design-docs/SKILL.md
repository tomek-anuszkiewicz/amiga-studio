---
name: sync-design-docs
description: Before every authorized Git commit, review the intended changes per crate and synchronize only the design specifications selected by tracked_paths; leave already accurate documents untouched.
---

# Design Documentation Review Before Commit

Follow [docs-maintenance.md](../../rules/docs-maintenance.md) and
[git-commits.md](../../rules/git-commits.md). This procedure reviews documentation;
it does not authorize staging, committing, or modifying unrelated work.

## When to Use

Mandatory before every authorized Git commit, including documentation, rules,
tests and merge commits. Review only the intended commit snapshot. Ordinary
implementation or refactoring alone does not trigger this procedure. Milestone
closure uses its completion commit's review; do not repeat it for the same
snapshot unless relevant inputs changed. Explicit review requests remain valid.

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
  they have no tracked paths. For changes solely to rules or tooling with no
  mapping, review their directly affected instructions and links, not all
  emulator specifications.
- Workspace/dependency changes require the affected crate manifests and
  [General Architecture](../../../Obsidian/Amiga/Design/General%20Architecture.md),
  not every implementation file.

## 3. Review the Selected Changes

Read each selected document and inspect only the intended changed files, their
diff and necessary local context. Evaluate behavior, registers, timing phases,
bus coordination, public contracts and dependency changes described by the
specification. Extend to another subsystem only when a concrete shared contract,
dependency or interaction requires it; record the reason and added scope.

Follow [spec-compliance.md](../../rules/spec-compliance.md) for conflicts. Do not
rewrite a specification simply to legitimize an implementation divergence.

## 4. Update Only Real Documentation Differences

Correct affected descriptions, register/timing tables and dependency diagrams.
Prune superseded drafts and copied implementations only within the reviewed
scope; retain concise links to living source. Update `tracked_paths` if ownership
changed. Design specifications have no line-count ceiling.

If the document already describes the intended code correctly, leave the file
unchanged, including its `updated` date. Change `updated` only alongside a real
content correction. Never add synchronization hashes/dates, periodic stamps or
review-only metadata. A successful review can produce no file edits and no
additional commit.

## 5. Validate and Return to the Commit Workflow

After document edits, follow
[obsidian-vault-linking](../obsidian-vault-linking/SKILL.md) for metadata and link
checks. Include only the necessary documentation corrections in the agreed
atomic commit. Recheck selection if intended code, ownership or the proposed
snapshot changes. The enclosing workflow runs verification on the final source
and stages only authorized paths; do not repeat its gates inside this skill.

## Report

Record the intended snapshot, changed crates/paths, selected documents, concrete
corrections or no-change findings, scope extensions, missing/invalid mappings,
and checks actually run. Do not claim unchanged documents were updated or that
an unreviewed subsystem was verified.

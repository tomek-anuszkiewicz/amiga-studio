# Documentation Maintenance

Reader-facing documentation under `docs/` explains the project and provides entry points for users and developers. Follow [information hierarchy](information-hierarchy.md) when selecting scope and detail. Keep operational procedures with their owning tools, tests or agent references; update incoming links when moving them.

## Living Design Specifications

The following requirements apply to specifications under `Obsidian/Amiga/Design/`.

- Before every authorized commit, use [sync-design-docs](../skills/sync-design-docs/SKILL.md) to review the intended changes per crate and reconcile only affected specifications. Ordinary implementation/refactoring alone does not trigger synchronization; milestone closure uses its completion commit's review. The review does not authorize a commit.
- Select documents using repository-relative `tracked_paths` in their frontmatter. Match exact files or directory descendants; include both sides of renames and deletions. Keep the mapping accurate and report missing ownership. Read selected diffs and necessary context, not all repository code or history.
- Establish the exact intended commit snapshot. If staged, review its index version; otherwise use only agreed paths/hunks and explicitly selected new files. Resolve ambiguous scope before proceeding. Include documentation corrections in the same atomic commit.
- If specifications already match the intended changes, leave them and their dates untouched. No synchronization hashes, synchronization dates, age/commit-count thresholds or review-only metadata are required.
- Read the authoritative specification through the [design index](../../Obsidian/Amiga/Design/General%20Architecture.md) and handle conflicts through [spec-compliance.md](spec-compliance.md); do not silently align documentation to incorrect code.
- Remove superseded proposals and draft code after implementation. Explain mechanisms and link living source rather than copying implemented Rust definitions.
- Describe shared microcode primitives once. Instruction-class sections contain their ordering, timing parameters and structural exceptions; link to shared phase definitions and the CPU specification for instruction semantics.
- Correct an existing class description or table before adding another section. Add a variant only when its execution structure differs, not merely because another mnemonic was implemented. Keep repair history, test results and implementation progress in task plans or the diary.
- Update the dependency graph in General Architecture when crate relationships change. Keep specifications and references complete; they have no line-count ceiling.
- Follow the vault metadata and link requirements below, [information hierarchy](information-hierarchy.md), and [writing style](practitioner-voice-and-tone.md).
- At milestones, review bidirectional semantic parity as required by [audit-semantic-parity](../skills/audit-semantic-parity/SKILL.md); mechanical drift checks alone do not prove agreement.

Shared script placement and instruction ownership are governed by [agent tooling maintenance](../references/agent-tooling.md#agent-tooling-maintenance). Specifications may be reached through the design index; neither a direct rule link for every document nor a manual rule audit registry is required.

## Vault Metadata and Link Integrity

These requirements apply to authored specifications under `Obsidian/Amiga/Design/`. Preserve the metadata contract when editing reference notes; raw conversion procedures remain documented separately.

- Frontmatter begins on line 1 and records title, aliases, tags, category, subsystem, status, created/updated dates, and related links as applicable. Use [obsidian-vault-linking](../skills/obsidian-vault-linking/SKILL.md) for the YAML schema and maintenance procedure.
- On a substantive content edit, update `updated` and evaluate status, tags, and related dependencies. A review without corrections leaves dates unchanged; mechanical removal of obsolete synchronization metadata does not require a date-only update.
- Maintain contextual inline links where relationships are explained and a curated final reference section with concise source rationale. Link relevant manuals, reference implementations, and living Rust modules when applicable.
- All on-disk links must resolve. Update incoming links whenever their target moves; use relative paths and verify depth from the actual source file.
- [General Architecture](../../Obsidian/Amiga/Design/General%20Architecture.md) is the design hub. Maintain reciprocal links for meaningful subsystem dependencies without artificial link quotas or repetitive lists.
- Adapt document sections to the subsystem rather than duplicating a fixed outline.

The owning skill provides frontmatter templates, path examples, and link-check commands.

## Reference Book Metadata

Each Markdown file inside a book directory under `Obsidian/Amiga/Reference/`
has line-one frontmatter with `title`, `book`, `chapter`, and `tags`. Use `book`
for the source book title or, for a composite test collection, its publication
identifier. Use `chapter` for the file's section or named unit, including a
table of contents or front matter. Replace an existing `manual` property with
`book` without changing its value.

Include `reference`, `book`, and `chapter` in `tags`. Keep existing relevant
tags and add specific topics supported by that file's headings and content,
such as a chip, interface, operation, or timing subject. Reuse established
lowercase tags with hyphens between words. Do not infer a topic from the book
title alone. Include `amiga` when the file concerns the Amiga. The commit
frontmatter gate checks structural metadata; topic selection requires review.

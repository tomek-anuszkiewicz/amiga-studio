# Documentation Maintenance

Reader-facing documentation under `docs/` explains the project and provides entry points for users and developers. Follow [document and instruction ownership](information-hierarchy.md) when selecting scope and detail. Keep operational procedures with their owning tools, tests or agent references; update incoming links when moving them.

## Living Design Specifications

The following requirements apply to specifications under `Obsidian/Amiga/Design/`.

- Before every authorized commit, use [verify-spec-compliance](../skills/verify-spec-compliance/SKILL.md) to check the intended changes against approved requirements and enrich only affected specifications with compatible implementation details. Ordinary implementation/refactoring alone does not trigger this review; milestone closure may reuse its broader audit as described by the skill. The review does not authorize a commit.
- Select affected specifications by frontmatter `tracked_paths`, including renamed/deleted paths; report unmapped changes.
- Establish the exact intended commit snapshot. If staged, review its index version; otherwise use only agreed paths/hunks and explicitly selected new files. Resolve ambiguous scope before proceeding. Include documentation corrections in the same atomic commit.
- Leave accurate specifications and their dates unchanged.
- Read the authoritative specification through the [design index](../../Obsidian/Amiga/Design/General%20Architecture.md) and handle conflicts through [spec-compliance.md](spec-compliance.md); do not silently align documentation to incorrect code.
- Distinguish [compatible documentation enrichment](spec-compliance.md#compatible-documentation-enrichment) from [requirement changes before implementation](spec-compliance.md#specification-changes-before-implementation). Review changed requirements against code and tests even when only documentation changed; unresolved differences block task completion and the commit. Documentation enrichment does not require an exhaustive description of implementation details.
- Follow the explanatory style of adjacent specifications and link the implementation.
- Keep implementation gaps and known defects to brief, truthful caveats linked to their owning [roadmap stage](../../ROADMAP.md). Keep diagnostics, remaining work, ownership and acceptance evidence together in the owning roadmap section; remove verified items there as work completes. Preserve hardware requirements and explanations of active behavior in their owning specifications.
- Reuse existing specification structure and links for shared behavior.
- Extend the relevant existing description using nearby conventions.
- Update the dependency graph in General Architecture when crate relationships change. Keep specifications and references complete; they have no line-count ceiling.
- Follow the vault metadata and link requirements below, [document and instruction ownership](information-hierarchy.md), and [writing style](practitioner-voice-and-tone.md).
- At milestones, review bidirectional semantic parity as required by [audit-semantic-parity](../skills/audit-semantic-parity/SKILL.md); mechanical drift checks alone do not prove agreement.

Shared script placement and instruction ownership are governed by [agent tooling maintenance](../references/agent-tooling.md#agent-tooling-maintenance). Specifications may be reached through the design index; neither a direct rule link for every document nor a manual rule audit registry is required.

## Vault Metadata and Link Integrity

These requirements apply to authored specifications under `Obsidian/Amiga/Design/`. Preserve the metadata contract when editing reference notes; raw conversion procedures remain documented separately.

- Frontmatter begins on line 1 and records title, aliases, tags, category, subsystem, status, created/updated dates, and related links as applicable. Use [obsidian-vault-linking](../skills/obsidian-vault-linking/SKILL.md) for the YAML schema and maintenance procedure.
- On a substantive content edit, update `updated` and evaluate status, tags, and related dependencies. A review without corrections leaves dates unchanged; mechanical removal of obsolete synchronization metadata does not require a date-only update.
- Maintain contextual inline links where relationships are explained and a curated final reference section with concise source rationale. Link relevant manuals, reference implementations, and living Rust modules when applicable.
- Preserve resolving relative links when moving documents.
- Use [General Architecture](../../Obsidian/Amiga/Design/General%20Architecture.md) as the hub; link meaningful dependencies reciprocally without quotas.
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

# Vault metadata and relationships

These criteria apply to authored Design notes and relevant Reference relationships,
under [documentation policy](../../../rules/docs-maintenance.md).
Each Design document begins on line one with frontmatter:

```yaml
---
title: "<Full Subsystem / Document Title>"
aliases: ["<Short Code>", "<Alternate Name>"]
tags: ["amiga", "design", "<subsystem-tag>"]
category: "Design"
subsystem: "<subsystem_name>"
status: "active"
created: YYYY-MM-DD
updated: YYYY-MM-DD
related: ["[SiblingDoc.md](SiblingDoc.md)"]
---
```

Use `active`, `completed` or `draft` status reflecting actual maturity. Include
`tracked_paths` where the specification owns source paths; retain its existing
file/directory ownership meaning. Update `updated` only with substantive content,
not a no-change review or mechanical obsolete-metadata removal. Review tags,
status and related dependencies for meaning, not merely structural presence.

Maintain two useful link layers: contextual inline links where a relationship is
explained and a final curated reference section. Source entries include a short
rationale explaining the hardware contract, model comparison or implementation
they support. Use the General Architecture hub and reciprocal meaningful
dependencies without link quotas. Adapt section structure to the subsystem.

Resolve every relative target from its source file's parent, including nested
Design folders. For a note directly under Design, sibling links use `File.md`,
manual links `../Reference/<book>/<file>.md`, and repository paths
`../../../crates/<crate>/...`. A note one directory deeper needs an extra
`../` to reach those roots. Preserve relative paths on document moves and encode
spaces correctly. Inline and final references are complementary, not duplicate
mandatory inventories.

Reference-book Markdown has line-one `title`, `book`, `chapter` and `tags`.
Use the source book/publication identifier, the file's named unit (including front
matter) and tags `reference`, `book`, `chapter`, plus existing supported topics.
Replace an existing `manual` property with `book` without changing its value.
Infer topics from the file's actual content, not book title alone; include `amiga`
when appropriate. Proposed book corrections retain their concrete approval
boundary. A checker validates structure, not substantive topic selection.

The full Design checker rejects empty tags and broken links as well as metadata
structure. The canonical checker scans every local Amiga Markdown file, including
ignored books. Retain both actual scopes and reuse only overlapping evidence;
neither structural success nor link counts establish hardware or semantic parity.


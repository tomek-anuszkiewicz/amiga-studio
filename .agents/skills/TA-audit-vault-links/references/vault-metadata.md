# Vault metadata and relationships

These criteria apply to authored Design notes and relevant Reference relationships.
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

Reference books use the [book metadata schema](../../../../tools/bootstrap/book-to-markdown/reference-book-metadata.md).
Preserve source-note metadata while editing. Raw conversion follows the
[nested bootstrap instructions](../../../../tools/bootstrap/AGENTS.md).

The full Design checker rejects empty tags and broken links as well as metadata
structure. The canonical checker scans every local Amiga Markdown file, including
ignored books. Retain both actual scopes and reuse only overlapping evidence;
neither structural success nor link counts establish hardware or semantic parity.

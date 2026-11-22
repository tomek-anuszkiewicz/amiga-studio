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

Choose `active`, `completed` or `draft` according to the document's actual
maturity. Include `tracked_paths` where the specification owns source paths
and retain its existing file and directory ownership rules. Change `updated`
only when the content changes substantively. A review without changes or
mechanical removal of obsolete metadata leaves the date in place. Check that
the tags, status and related dependencies mean what the document needs them
to convey.

Link to related material where the text explains the relationship and keep a
curated reference section at the end. For each source entry, briefly explain
what hardware contract, model comparison or implementation it supports. Use
the General Architecture hub and link meaningful dependencies in both
directions. Let the relationships determine the number of links. Adapt the
section structure to the subsystem.

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

The full Design checker rejects invalid metadata structure, empty tags and
broken links. The canonical checker scans every local Amiga Markdown file,
including ignored books. Record the actual scope of each checker and reuse
evidence only where the scopes overlap. Structural success and link counts
are insufficient evidence of hardware or semantic parity.

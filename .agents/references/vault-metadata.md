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

Reference books use the [book metadata schema](../../tools/bootstrap/book-to-markdown/reference-book-metadata.md).
Preserve source-note metadata while editing. Raw conversion follows the
[nested bootstrap instructions](../../tools/bootstrap/AGENTS.md).

The recursive Design audit checks that each note's frontmatter yields at least
one parsed `tags` entry and that the local file paths it extracts from Markdown
links exist. [Design audit](../skills/TA-WF-Step-review-docs/scripts/audit_vault.py)

The canonical frontmatter check scans every local Amiga Markdown file, including
ignored books. It looks for opening and closing delimiters, text after `title:`,
and a `tags` property, and applies additional rules to reference book notes.
[Canonical frontmatter check](../../tools/verification/checks/check_obsidian_frontmatter.py)

During review, check YAML syntax and assess the meaning of metadata and links,
including blank tag entries and link fragments. Record each check's scope and
reuse evidence only where the scopes overlap. Assess hardware behavior and
semantic parity separately from these structural checks.

# Vault Metadata and Link Integrity

Applies to authored specifications under `Obsidian/Amiga/Design/`. Preserve the metadata contract when editing reference notes; raw conversion procedures remain documented separately.

- Frontmatter begins on line 1 and records title, aliases, tags, category, subsystem, status, created/updated dates, and related links as applicable. Code-backed checkpoint requirements belong to [docs-maintenance.md](docs-maintenance.md).
- On each edit, update the date and evaluate status, tags, and related dependencies. Use [obsidian-vault-linking](../skills/obsidian-vault-linking/SKILL.md) for the YAML schema and maintenance procedure.
- Maintain contextual inline links where relationships are explained and a curated final reference section with concise source rationale. Link relevant manuals, reference implementations, and living Rust modules when applicable.
- All on-disk links must resolve. Update incoming links whenever their target moves; use relative paths and verify depth from the actual source file.
- [General Architecture](../../Obsidian/Amiga/Design/General%20Architecture.md) is the design hub. Maintain reciprocal links for meaningful subsystem dependencies without artificial link quotas or repetitive lists.
- Follow [information hierarchy](information-hierarchy.md) and [practitioner style](practitioner-voice-and-tone.md); adapt document sections to the subsystem rather than duplicating a fixed outline.

The owning skill provides frontmatter templates, path examples, and link-check commands.

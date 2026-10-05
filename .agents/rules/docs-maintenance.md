# Design Documentation Maintenance

Applies to living specifications under `Obsidian/Amiga/Design/`.

- Update affected specifications whenever architectural decisions, timing models, data structures, register fields, or hardware quirks change or are clarified. Read the authoritative specification through the [design index](../../Obsidian/Amiga/Design/General%20Architecture.md); handle conflicts through [spec-compliance.md](spec-compliance.md).
- At minor roadmap points and genuine architectural changes, use [sync-design-docs](../skills/sync-design-docs/SKILL.md) to reconcile affected code and specifications. Checkpoint fields are `tracked_paths`, `last_synced_commit`, and `last_synced_date`; update them with substantive documentation changes.
- Routine changes that do not alter documented behavior may remain within the existing 100-commit / 30-day audit window. Exceeding either threshold requires review, not automatic checkpoint stamping. Conceptual notes without code-backed paths are exempt.
- Remove superseded proposals and draft code after implementation. Explain mechanisms and link living source rather than copying implemented Rust definitions.
- Update the dependency graph in General Architecture when crate relationships change. Keep specifications and references complete; they have no line-count ceiling.
- Follow [vault linking](vault-linking-and-graph-integrity.md), [information hierarchy](information-hierarchy.md), and [writing style](practitioner-voice-and-tone.md).
- At milestones, review bidirectional semantic parity through the skill required by [AGENTS.md](../../AGENTS.md); mechanical drift checks alone do not prove agreement.

Shared script placement and skill catalogs are governed by [agent tooling maintenance](../../docs/ai_agents.md#agent-tooling-maintenance). Specifications may be reached through the design index; neither a direct rule link for every document nor a manual rule audit registry is required.

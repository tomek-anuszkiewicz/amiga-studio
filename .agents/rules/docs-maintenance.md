# Design Documentation Maintenance

Applies to living specifications under `Obsidian/Amiga/Design/`.

- Before every authorized commit, use [sync-design-docs](../skills/sync-design-docs/SKILL.md) to review the intended changes per crate and reconcile only affected specifications. Ordinary implementation/refactoring alone does not trigger synchronization; milestone closure uses its completion commit's review. The review does not authorize a commit.
- Select documents using repository-relative `tracked_paths` in their frontmatter. Match exact files or directory descendants; include both sides of renames and deletions. Keep the mapping accurate and report missing ownership. Read selected diffs and necessary context, not all repository code or history.
- Establish the exact intended commit snapshot. If staged, review its index version; otherwise use only agreed paths/hunks and explicitly selected new files. Resolve ambiguous scope before proceeding. Include documentation corrections in the same atomic commit.
- If specifications already match the intended changes, leave them and their dates untouched. Update `updated` only with substantive content corrections. No synchronization hashes, synchronization dates, age/commit-count thresholds or review-only metadata are required.
- Read the authoritative specification through the [design index](../../Obsidian/Amiga/Design/General%20Architecture.md) and handle conflicts through [spec-compliance.md](spec-compliance.md); do not silently align documentation to incorrect code.
- Remove superseded proposals and draft code after implementation. Explain mechanisms and link living source rather than copying implemented Rust definitions.
- Update the dependency graph in General Architecture when crate relationships change. Keep specifications and references complete; they have no line-count ceiling.
- Follow [vault linking](vault-linking-and-graph-integrity.md), [information hierarchy](information-hierarchy.md), and [writing style](practitioner-voice-and-tone.md).
- At milestones, review bidirectional semantic parity as required by [audit-semantic-parity](../skills/audit-semantic-parity/SKILL.md); mechanical drift checks alone do not prove agreement.

Shared script placement and instruction ownership are governed by [agent tooling maintenance](../../docs/ai_agents.md#agent-tooling-maintenance). Specifications may be reached through the design index; neither a direct rule link for every document nor a manual rule audit registry is required.

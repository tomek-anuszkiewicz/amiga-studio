# Roadmap and Milestone Policy

- [ROADMAP.md](../../ROADMAP.md) is the active backlog. Remove fully implemented and verified steps from Section 2; do not retain completed tags, checked boxes, strikethrough, or execution history there.
- For major capabilities, update the concise baseline in Section 1. Keep historical detail in diary and Git, and renumber remaining steps coherently.
- Plan by physical dependencies: clocks and bus arbitration, autonomous DMA/coprocessors, video serialization, peripherals, then firmware integration. Do not derive priority from folder order.
- A minor roadmap point or major milestone completes only after the milestone gate and semantic parity review required by [AGENTS.md](../../AGENTS.md), affected design synchronization, checkpoint review, diary compaction, and backlog pruning.
- Use [roadmap-maintenance](../skills/roadmap-maintenance/SKILL.md) for completion evidence, pruning, renumbering, and scorecard updates; [docs-maintenance.md](docs-maintenance.md) and [diary-maintenance.md](diary-maintenance.md) own their respective obligations.
- Refresh the [vAmigaTS Verification Scorecard](../../Obsidian/Amiga/Design/vAmigaTS%20Verification%20Scorecard.md) after a roadmap sub-suite completes, a major timing fix changes global pass rates, or the user requests a status audit. Keep transient debug percentages out of the roadmap.

---
name: TA-organize-roadmap
description: Consolidate and order selected pending roadmap work without deleting it as completed.
---

# Organize the selected pending work

Apply [the invocation contract](../../references/workflow-contract.md) and
[roadmap policy](../../rules/roadmap-maintenance.md).
Executor: main session; result kind: `documents`.
Inputs: selected pending sections, user priorities and known dependencies.

1. Read the assigned scope and distinguish pending requirements from project
   descriptions, methods and history. Preserve uncertain completion as pending.
2. Consolidate related objectives and remove duplicate wording while retaining
   substantive requirements, acceptance evidence, failure handling and decisions.
   Detailed execution plans remain under `.agent/tasks/`.
3. Order work by user priorities and physical dependencies: bus/clocks, DMA,
   display, peripherals, firmware. Preparatory conversion may precede implementation.
4. Renumber and update affected references. Run the shared structural check:
   `python .agents/skills/TA-prune-roadmap/scripts/check_roadmap.py`.
5. Return reorganization, retained scope, next actions and actual checks.

Stop after organization. Do not switch to completion-based pruning, start audits,
compact the diary or claim stage completion.

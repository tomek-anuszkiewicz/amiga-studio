---
name: TA-prune-roadmap
description: Remove selected completed roadmap work using supplied completion evidence while preserving remaining scope.
---

# Prune the selected completed work

Apply [the invocation contract](../../references/workflow-contract.md) and
[roadmap policy](../../rules/roadmap-maintenance.md).
Executor: main session; result kind: `documents`.
Inputs: exact selected items, implementation/semantic/verification evidence,
closure decisions, checkpoint state and candidate.

1. Evaluate supplied completion evidence and actual remaining scope. A plan,
   prepared artifact directory or source change alone does not prove completion.
   Report missing evidence without obtaining it automatically.
2. Prepare removal of the verified completed scope entirely. Preserve distinct
   unfinished requirements with concrete gates; history belongs in diary/Git,
   not a completed-capability section. Preserve user priorities and dependencies.
3. Update numbering, references and assigned checkpoints. Run
   `python .agents/skills/TA-prune-roadmap/scripts/check_roadmap.py`.
4. Return removed/retained items, supporting evidence, checks and missing closure
   prerequisites. Pruning remains provisional until required final gates, semantic
   review, diary preparation/compaction and the closure commit succeed.

Stop after pruning/refusal. Do not run verification, audits, diary operations,
commits or publication inside this operation.


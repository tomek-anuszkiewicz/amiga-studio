---
name: TA-optimize-emulation
description: Implement explicitly selected optimization candidates within bounded rounds and compare the final result with the original baseline.
---

# Implement the selected optimization scope

Apply [the invocation contract](../../references/workflow-contract.md).
Role: `TA-implementer`; result kind: `implementation`.
Inputs: selected candidate IDs or concrete bounded saving mechanism, approved
behavior/test scenarios, original source/build baseline, workloads and write scope.

1. Read relevant advance requirements and the
   [measurement contract](../TA-profile-emulation/references/optimization-evidence.md).
   Validate current evidence and preserve original baseline/build/symbols before
   mutation. Missing necessary baseline is a blocker, not permission to fabricate one.
2. Implement one coherent selected change with the simplest mechanism removing
   repeated work. Explain dependencies/effective update points and preserved behavior;
   reject hidden backchannels, scattered invalidation and parallel sources of truth.
3. Run focused correctness checks before repeated comparable unprofiled A/B timing.
   Include affected active/idle/transitional cases and maintenance costs. Each
   implementation/repair pass with checks/measurement is one declared round.
4. Keep a change only when evidence supports its benefit, behavior and simplicity.
   Report regressions/tradeoffs and inconclusive results honestly. Remove only this
   experiment's rejected edits, preserving unrelated and previously accepted work.
   A further uncertainty investigation needs a concrete approach within the budget.
5. Update the same `micro-optimizations.md` candidate report. Compare the final
   accepted combination directly with the original baseline; never sum individual
   percentages. Return changes, checks, measurements and unresolved opportunities.

Stop at the assigned scope or limit. Do not select another candidate outside the
assignment, launch broad profiling, loosen correctness criteria, stage or commit.


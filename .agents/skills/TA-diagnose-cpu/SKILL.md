---
name: TA-diagnose-cpu
description: Reproduce a selected 68000 vector mismatch and report state, instruction-clock and bus-callback evidence without repairs.
---

# Diagnose the selected CPU case

Apply [the invocation contract](../../references/workflow-contract.md).
Role: `TA-cpu-verifier`; result kind: `cpu_diagnosis`.
Inputs: selected vector/case or CPU evidence question, exact candidate, available
reproduction, requested sample/full scope and diagnostic output directory.

1. Read the [CPU verification specification](../../../Obsidian/Amiga/Design/Verification/CPU%20SingleStepTests.md)
   and relevant requirements. Use an existing isolated failing vector when sufficient;
   otherwise reproduce only the selected scope with
   `python tools/verification/verify.py test runner singlestep case <filter> sample <count>`.
   Use `full` only when that complete group was assigned.
2. Report final-state, instruction-clock and timed bus-callback differences
   separately. Final-state evidence cannot establish the exact cycle of a change.
3. Trace relevant micro-steps, prefetch, effective-address updates and condition
   codes toward the cause. Distinguish CPU defect, harness limit, approved override
   and specification conflict; retain unknown causes as hypotheses.
4. Return actual command/selection/results, mismatch records, diagnosis, evidence
   and limits. Software-generated vectors exercise an isolated CPU fixture, not
   the production Amiga bus or physical silicon directly.

Stop after diagnosis. Normal tool diagnostics/results are allowed; no source,
test or reference repairs, new regression tests, automatic fix, full-suite
escalation or claims that a targeted pass verifies the entire dataset.

# Diagnosing an isolated 68000 mismatch

Read the [CPU verification specification](../../Obsidian/Amiga/Design/Verification/CPU%20SingleStepTests.md)
and the requirements for the affected behavior. Reuse an isolated failing vector
if it provides enough evidence. Otherwise, use the command below to reproduce
only the selected group or sample.
[Verification frontend](../../tools/verification/README.md)

```text
python tools/verification/verify.py test runner singlestep case <filter> sample <count>
```

Use `full` only when the assignment explicitly covers an entire group or
suite. Report final-state differences, total instruction clocks and timed bus
callbacks separately. The final state records the result at the end of execution.
The exact clock of each change remains undetermined from the final state alone. Trace relevant micro-steps, prefetch,
effective-address updates and condition codes. Distinguish CPU defects, harness
limits, approved overrides and specification conflicts. Report possible causes
that lack supporting evidence as hypotheses.

Preserve the command as run, the selected vectors, the mismatch records and the
limits of the diagnosis. A passing targeted run establishes the result for the
vectors it ran. Software-generated vectors exercise an isolated CPU fixture,
so evidence from that fixture leaves production Amiga arbitration and physical
silicon behavior unverified. Diagnosis alone gives no authority to repair
source code, tests or references or to change expected results.

# Diagnosing an isolated 68000 mismatch

Read the [CPU verification specification](../../Obsidian/Amiga/Design/Verification/CPU%20SingleStepTests.md)
and affected behavior requirements. Reuse an existing isolated failing vector
when sufficient; otherwise reproduce only the selected group/sample through the
[verification frontend](../../tools/verification/README.md):

```text
python tools/verification/verify.py test runner singlestep case <filter> sample <count>
```

Use `full` only for an explicitly assigned complete group or suite. Report
final-state differences, total instruction clocks and timed bus callbacks
separately: final state cannot establish the exact cycle of a change. Trace
relevant micro-steps, prefetch, effective-address updates and condition codes.
Distinguish CPU defects, harness limits, approved overrides and specification
conflicts; retain unsupported causes as hypotheses.

Preserve the actual command/selection, mismatch records and diagnostic limits.
Software-generated vectors exercise an isolated CPU fixture, not production
Amiga arbitration or physical silicon. A targeted pass does not verify the full
dataset. Diagnosis alone does not authorize source/test/reference repairs or
changes to expected results.

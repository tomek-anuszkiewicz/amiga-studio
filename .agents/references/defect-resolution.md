# Defect reproduction and causal repair

Load this procedure before changing behavior to repair a suspected defect. It is
shared by assigned implementation and repair skills; it does not select another
operation or grant a larger scope.

1. Trace the failing data flow, clock phase or state transition toward its cause.
   Establish the failure on the original code with a focused test, reproducible
   case, trace or measurement suited to the issue. Record what it establishes and
   distinguish supported causes from hypotheses.
2. Repair the demonstrated mechanism within the assignment. Change a shared
   upstream mechanism only when evidence identifies it as the cause; reuse its
   authoritative constants, types and helpers. Preserve unrelated work.
3. Verify the original reproduction or measurement and relevant adjacent
   regressions. Keep repeatable protection suited to the defect under
   [testing policy](../rules/unit-testing-policy.md). Planned fixture and assertion
   repairs are in scope; newly discovered scenarios beyond the agreed test plan
   require a caller decision before implementation.
4. Review the final diff for technical necessity, duplicate mechanisms and complete
   caller migration. Report delivered changes, actual checks and unresolved limits.

Do not nudge coordinates, clock delays or beam offsets to fit a test, hardcode
one-artifact OCR substitutions, or special-case an opcode, address or filename to
hide the cause. Model the actual condition in the state machine or data model.
Never change goldens or timing references to manufacture a pass. Apply
[specification compliance](../rules/spec-compliance.md), including its separate
evidence-based exception for provisional register mutation parameters.

CPU vector divergences additionally use the
[CPU verification specification](../../Obsidian/Amiga/Design/Verification/CPU%20SingleStepTests.md).
Focused commands and saved reports belong to the
[verification reference](../../tools/verification/README.md).
Ordinary repair completion does not start language scans, broad coverage, graph
refresh, batch preparation or milestone work. An assessment reports the need for
a repair; only a separately assigned writer performs it.

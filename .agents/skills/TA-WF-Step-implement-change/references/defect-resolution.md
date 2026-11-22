# Defect reproduction and causal repair

Read this procedure before changing behavior to repair a suspected defect. The
assigned implementation and repair skills share it. Keep the operation and
scope of the current assignment.

1. Trace the failing data flow, clock phase or state transition to investigate
   its cause. Demonstrate the failure on the original code with a focused test,
   reproducible case, trace or measurement suited to the issue. Record what the
   evidence shows and which proposed causes remain hypotheses.
2. Repair the mechanism the evidence identifies, within the assignment. Change
   a shared upstream mechanism only when the evidence shows it causes the failure.
   Reuse its authoritative constants, types and helpers. Preserve unrelated work.
3. Repeat the original reproduction or measurement and run relevant regression
   checks for nearby behavior. Keep a repeatable check suited to the defect
   under the [testing criteria](../../../references/testing-criteria.md).
   The assignment permits planned fixture and assertion repairs. If you
   discover scenarios beyond the agreed test plan, obtain the caller's
   decision before implementing them.
4. Review the final diff for technical necessity, duplicate mechanisms and complete
   caller migration. Report delivered changes, actual checks and unresolved limits.

Do not nudge coordinates, clock delays or beam offsets to fit a test, hardcode
one-artifact OCR substitutions, or special-case an opcode, address or filename to
hide the cause. Model the actual condition in the state machine or data model.
Never change goldens or timing references to manufacture a pass. Apply
[specification compliance](../../../references/spec-compliance.md), including its separate
evidence-based exception for provisional register mutation parameters.

CPU vector divergences additionally use the
[CPU verification specification](../../../../Obsidian/Amiga/Design/Verification/CPU%20SingleStepTests.md).
Focused commands and saved reports belong to the
[verification reference](../../../../tools/verification/README.md).
Ordinary repairs stop before language scans, broad coverage, graph refresh,
batch preparation or milestone work. The assessor reports the need for a repair.
Only a separately assigned writer makes it.

---
name: TA-verify-milestone
description: Execute the canonical milestone profile at its authorized trigger and report evidence without closure edits.
---

# Verify the selected milestone

Apply [the invocation contract](../../references/workflow-contract.md),
[milestone completion criteria](../../../Obsidian/Amiga/Design/Verification/Testing%20Strategy%20and%20Quality%20Assurance.md#roadmap-and-milestone-completion) and the
[verifier contract](../../../tools/verification/README.md#completion-verification).
Executor: main session; result kind: `verification`; profile: `milestone`.
Inputs: actual milestone scope/trigger, exact candidate and results directory,
plus applicable immutable integration selectors.

1. Confirm required local inputs and candidate stability.
2. Run `python tools/verification/verify.py milestone` with the supplied
   integration selectors when applicable. Preserve full SingleStep coverage.
   The commit profile and the published English audit run at their own stages.
3. Await and inspect the canonical verdict, execution selection, retained groups
   and intact evidence. Return actual results and unrun/failed work.

Stop after verification. A pass alone does not close a stage. Do not prune roadmap
work, compact the diary, repair source, start audits, commit or publish.

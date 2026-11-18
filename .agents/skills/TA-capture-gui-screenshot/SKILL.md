---
name: TA-capture-gui-screenshot
description: Run one existing headless GUI capture for a supplied scenario and return the produced image and path.
---

# Capture one GUI frame

Apply [the invocation contract](../../references/workflow-contract.md).
Executor: main session or assigned specialist; result kind: `capture`.
Inputs: scenario, output path and optional width/height. Default scenario is
`baseline`; use only scenarios supported by the current inspector.

1. Run the existing tool with the supplied capture inputs:
   `python tools/verification/verify.py runner gui -- --scenario <scenario> --output <path>`.
   Forward optional `--width`, `--height` and assigned `--json-metadata` paths.
2. Confirm the command exit status and that it produced the requested image.
   Return the image, repository-relative path, dimensions/scenario when exposed,
   command evidence and actual error or limits.
3. Persist the matching text/JSON result and stop.

Capture has no mandatory interpretation, visual-quality checklist, diagnosis,
repair, tests, delegation or escalation. The caller's assignment may separately
require inspecting the image. This skill adds no renderer, wrapper or automatic
GUI-repair workflow.

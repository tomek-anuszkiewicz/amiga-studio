---
name: TA-capture-gui-screenshot
description: Run one existing headless GUI capture for a supplied scenario and return the produced image and path.
---

# Capture one GUI frame

Apply [the result contract](../../references/workflow-contract.md).
The main session or an assigned specialist runs the capture. Supply a scenario
and output path, with width and height if requested. Use only scenarios the
current inspector supports. The default is `baseline`.

1. Run the existing tool with the supplied capture inputs:
   `python tools/verification/verify.py runner gui -- --scenario <scenario> --output <path>`.
   Forward optional `--width`, `--height` and assigned `--json-metadata` paths.
2. Check the command's exit status and confirm that it produced the requested
   image. Return the image, its repository-relative path, the command evidence
   and any actual error or limit. Include the dimensions and scenario when
   the tool exposes them.
3. Return the matching text/JSON result and stop.

Capture has no mandatory interpretation, visual-quality checklist, diagnosis,
repair, tests, delegation or escalation. The caller's assignment may separately
require inspecting the image. This skill adds no renderer, wrapper or automatic
GUI-repair workflow.

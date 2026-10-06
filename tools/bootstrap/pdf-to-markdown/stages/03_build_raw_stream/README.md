# Stage 03: Legacy Raw Stream Assembly

This worker is unavailable because its segmentation input was removed.
Stage 02d emits `md_text`, pixel boxes and new object types; the retained
stream workers expect `raw_text` and PDF-point geometry. Assembly and its
consumers require a separate redesign before Stages 03-14 can execute.
Use the orchestrator with `--to-stage 02m` for the current page-review workflow.
The worker raises an explicit error rather than consuming stale segmentation
or silently converting the new contract into the old one.

The shared `extract_initial_assets.py` cropper remains for the legacy Stage 04
implementation. No new conversion of page objects into old nodes is provided.

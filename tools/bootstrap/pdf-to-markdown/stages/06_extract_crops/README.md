# Stage 06: Extract Graphic Crops

Read selected Stage 04 page objects, using the Stage 05 full-page replacement
where one exists. Extract every `graphic` from its original Stage 02 PNG using
the existing `bbox`: top-left pixel coordinates with exclusive upper bounds.
Save each crop as `06_extract_crops/assets/<segment_id>.png`.

No model calls, reclassification, padding, rotation or text conversion occur.
Source PNGs and segment JSONs remain unchanged. The filename preserves the
source segment identity; this stage emits assets only, with no additional JSON.
Later book processing copies these assets into its generated version directories.

Run through the [pipeline](../../README.md) with `--from-stage 06 --to-stage 06`
after successful Stages 02, 04 and 05. `--page-ranges` applies to this invocation;
omitting it processes all available Stage 04 pages. The status records the range.
Pipeline restart removes the Stage 06 directory before rebuilding, including
previous crops, while preserving predecessor outputs.

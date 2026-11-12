# Stage 06: Extract Graphic Crops

Read selected page objects from the latest available JSON in Stages 05, 04 and 03,
in that order. Omit pages with empty `segments` in their latest JSON.
Extract every `graphic` from its original Stage 02 PNG using
the existing `bbox`: top-left pixel coordinates with exclusive upper bounds.
Save each crop as `06_extract_crops/assets/<segment_id>.png`.

No model calls, reclassification, padding, rotation or text conversion occur.
Source PNGs and segment JSONs remain unchanged. The filename preserves the
source segment identity; this stage emits assets only, with no additional JSON.
Later book processing copies these assets into its generated version directories.

The shared [preparation cache](../../README.md#preparation-cache) stores each crop
under `tools/bootstrap/.cache/pdf/06_extract_crops/<SHA-256>/`. Its key includes
the source page PNG SHA-256, crop `bbox`, runtime identity including Pillow, and
the crop contract version. It excludes workspace paths and segment IDs.
Cache hits restore the crop to its current segment filename and print `[cache]`.
Changed source images or boxes require new crops; incomplete or damaged cache
entries are recomputed. Pipeline cleanup leaves this cache intact.

Run through the [pipeline](../../README.md) with `--from-stage 06 --to-stage 06`
after successful Stages 02, 04 and 05. `--page-ranges` applies to this invocation;
omitting it processes all retained pages. The status records the range.
Without `--page-ranges`, pipeline restart removes the Stage 06 directory before
rebuilding. With a range, it clears only crops for the selected pages. Earlier
stage outputs remain unchanged.

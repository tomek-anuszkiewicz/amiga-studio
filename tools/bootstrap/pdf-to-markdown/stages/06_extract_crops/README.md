# Stage 06: Extract Graphic Crops

For each selected page, Stage 06 looks for page objects in Stages 05, 04 and
03, in that order. It skips the page if the latest available JSON has an
empty `segments` list. It uses each `graphic` object's existing `bbox` to crop
the page's original Stage 02 PNG. The rectangle uses pixel coordinates from the
top-left corner and excludes the upper bounds. The stage saves each crop as
`06_extract_crops/assets/<segment_id>.png`.

The stage makes no model calls and performs no reclassification, padding,
rotation or text conversion. It leaves the source PNGs and segment JSON files
unchanged. It writes only assets, with each filename preserving the source
segment identity.
Later book processing copies these assets into its generated version directories.

The shared [preparation cache](../../README.md#preparation-cache) stores each crop
under `tools/bootstrap/.cache/pdf/06_extract_crops/<SHA-256>/`. Its key includes
the source page PNG SHA-256, crop `bbox`, runtime identity including Pillow, and
the crop contract version. It excludes workspace paths and segment IDs.
When the stage finds a matching, complete cache entry, it restores the crop
using the current segment filename and prints `[cache]`. It computes a new
crop if the source image or box changed or the cache entry is incomplete or
damaged. Pipeline cleanup leaves this cache intact.

Run through the [pipeline](../../README.md) with `--from-stage 06 --to-stage 06`
after successful Stages 02, 04 and 05. `--page-ranges` selects pages for this
invocation. Omit it to process all retained pages. The status records the range.
Without `--page-ranges`, pipeline restart removes the Stage 06 directory before
rebuilding. With a range, it clears only crops for the selected pages. Earlier
stage outputs remain unchanged.

# Stage 03: Build Raw Stream

Consume validated `02_page_conversion/page_NNNN_segments.json` objects and
Stage 01 PNG/text pairs. Preserve physical page order and each segment array's
reading order. Stage 02.5 review images are not inputs.

Each node keeps `segment_id`, `type`, `heading_level`, `continuation`, `md_text`
and the original integer `bbox_pixels`. Copy `md_text` into `raw_text` for the
existing downstream consumers; that field contains Markdown, not raw OCR.
Leave `rendered_markdown` unset so later workers retain their existing behavior.
Compute `bbox_norm` against the original PNG dimensions, then scale by displayed
page width/height to produce the point-based `bbox` used by the existing cropper.
This conversion matches that cropper's actual-image scaling, rather than
assuming that configured DPI exactly predicts rounded raster dimensions.

Validate the complete selected page set before creating output. Emit
`03_build_raw_stream/raw_stream.json` and crop visual assets for table, graphic
and code-block nodes into `03_build_raw_stream/assets/`. The shared cropper keeps
its existing padding and neighbor limits. All new object types remain intact;
this stage does not change classifications or reformat source Markdown.

Run through the [orchestrator](../../README.md):

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 03 --to-stage 03
```

Stage 03 can run with validated 00/01/02 completion records, even when 02.5
has not run. Restart 02 invalidates both the review and stream branches;
restart 02.5 invalidates only the review.

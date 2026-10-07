# Stage 03: Build Raw Stream

Consume retained `02.8_filter_page_content/page_NNNN_segments.json` objects and
Stage 01 PNG/text pairs. Preserve physical page order and each segment array's
reading order. Stage 02.5 review images are not inputs. Stage 02.8 removes
headers/footers, pre-TOC objects when a boundary exists, and entire list/index
pages before nodes or assets are assembled. There is no fallback to Stage 02;
an empty filtered input produces an empty node stream and assets directory.

Each node keeps `segment_id`, `type`, `heading_level`, `continuation`, `md_text`
and the original integer `bbox_pixels`. Copy `md_text` into `raw_text` for the
existing downstream consumers; that field contains Markdown, not raw OCR.
Leave `rendered_markdown` unset so later workers retain their existing behavior.
Compute `bbox_norm` against the original PNG dimensions, then scale by displayed
page width/height to produce the point-based `bbox` used by the existing cropper.
This conversion matches that cropper's actual-image scaling, rather than
assuming that configured DPI exactly predicts rounded raster dimensions.

Read existing page artifacts in numeric physical-page order, without coverage,
pair-presence, schema or geometry checks. Emit
`03_build_raw_stream/raw_stream.json` and crop visual assets for table, graphic
and code-block nodes into `03_build_raw_stream/assets/`. The shared cropper keeps
its existing padding and neighbor limits. All new object types remain intact;
this stage does not change classifications or reformat source Markdown.

Run through the [orchestrator](../../README.md):

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 03 --to-stage 03
```

Stage 03 requires successful Stage 01 and 02.8 execution statuses, even when
02.5 or 02.9 has not run. Restart 02, 02.5, 02.8 or 02.9 clears Stage 03 and all
later stages, because cleanup follows execution order rather than artifact
dependencies. Existing 02.9/03 successes require regeneration starting at 02.8
to reflect filtering.

# Stage 02.82: Table Conversion Review

Deterministic review of completed Stage 01 and 02.81 artifacts, without inference
or Stage 02.9 output. Produce one `<segment_id>_review.png` per table, in physical
page and source order, and report paths and counts by HTML/Markdown/unconverted.

The Source panel shows original Stage 01 pixels for the table's recorded source
group, using the persistent Stage 02.81 table crop at its original location. The
Converted panel uses the same group assembly as page and stream export. It renders
GFM and HTML tables, including cell spans, associated blocks in source order and
the saved literal RAG text. Only the textual companion is expanded in the review
DOM; the following original-image companion remains collapsed. Both are collapsed
in exported documents. Labels identify the physical page, segment ID and format.
Unconverted tables show their retained raster with an explicit Unconverted label.

Rendering uses `markdown-it-py` and Playwright with an installed Microsoft Edge
headless backend (`table_review.browser_channel: msedge`). Install shared Python
requirements. Select `chrome` for installed Chrome, or `chromium` after
`python -m playwright install chromium`. Browser selection is explicit; launch
failures stop review. Browser support and screenshot APIs are documented by
[Playwright](https://playwright.dev/python/docs/browsers) and its
[screenshot guide](https://playwright.dev/python/docs/screenshots).

Temporary HTML lives separately from final PNGs and is removed after capture.
The full-page screenshot expands to document width and height, retaining long literal
lines and complete rows. Source aspect ratio is preserved; original PNGs and crops
are never modified. Review disables page scripts and remote HTTP requests.

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 02.82 --to-stage 02.82
```

Restart clears 02.82 and all later results, preserving 01/02/02.8/02.81. Review
PNGs are neither document assets nor conversion inputs: 02.9 and 03 require
completed 02.81 directly. A stop at 02.82 does not export or build the stream.
Use compatible existing artifacts for a user-requested review-only run; do not
repeat inference or choose extra fragments. Rendering success proves production
of the comparison; the user assesses source fidelity. RAG ingestion of details
and code-block content remains a separate deferred task.

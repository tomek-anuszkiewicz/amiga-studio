# Stage 03: Page Objects and Markdown

For each selected Stage 02 page, send its original-resolution PNG and matching
text JSON to Codex. The [prompt](prompt.md) defines grouping, classification,
reading order, transcription and the returned JSON fields. No output schema or
separate type enumeration is sent to the model; the prompt is the authority for
classification, including `legend`, `note` and `preformatted_text`.

Write `03_page_conversion/page_XXXX_segments.json` with ordered logical objects,
their types, `md_text` and integer pixel boxes `[x0,y0,x1,y1]`. Boxes use the
original PNG's top-left origin and exclusive upper bounds. Assign page-scoped
`segment_id` values in returned order. Saved objects omit empty/default fields.

Table and graphic transcription is deferred; Stage 05 transcribes retained
tables. [Stage 04](../04_filter_page_content/README.md) filters the saved objects.
Changing the prompt does not rewrite existing results.

Run through the [pipeline](../../README.md) with `--from-stage 03 --to-stage 03`
after successful Stage 02. Use the attempt's configured model and effort.

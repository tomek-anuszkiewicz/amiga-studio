# Stage 03: Page Objects and Markdown

For each selected Stage 02 page, send its original-resolution PNG and matching
text JSON to Codex using the attempt's configured model and effort. The
[prompt](prompt.md) is the sole source of inference instructions and response
format. No separate output schema or type enumeration is sent.

The program assigns page-scoped `segment_id` values in returned order and saves
the response to `03_page_conversion/page_XXXX_segments.json`. The shared artifact
writer omits empty/default fields.

[Stage 04](../04_filter_page_content/README.md) consumes the saved page objects.
Changing the prompt does not rewrite existing results.

Run through the [pipeline](../../README.md) with `--from-stage 03 --to-stage 03`
after successful Stage 02.

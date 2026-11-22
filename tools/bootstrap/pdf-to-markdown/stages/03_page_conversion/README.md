# Stage 03: Page Objects and Markdown

Stage 03 sends the original-resolution PNG and matching text JSON for each
selected Stage 02 page to Codex. It uses the model and effort configured for
the attempt. The [prompt](prompt.md) provides all inference instructions and
defines the response format. The stage sends no separate output schema or type
list.

The program assigns `segment_id` values within each page, in the order Codex
returns the objects. It saves the response to
`03_page_conversion/page_XXXX_segments.json`. The shared component that writes
the file omits empty fields and fields with default values.

The configured `llm.concurrency` limits how many pages can run at once. Each
worker uses a separate Codex client, and the coordinator saves each completed
page to its own file.

[Stage 04](../04_filter_page_content/README.md) consumes the saved page objects.
Changing the prompt does not rewrite existing results.

Run through the [pipeline](../../README.md) with `--from-stage 03 --to-stage 03`
after successful Stage 02.

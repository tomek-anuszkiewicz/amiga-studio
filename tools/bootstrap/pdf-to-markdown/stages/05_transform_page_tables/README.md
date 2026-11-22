# Stage 05: Transform Page Tables

Stage 05 reads the selected pages from Stage 04 JSON and uses the original
Stage 02 PNGs. It reads unchanged pages from Stage 03 and skips pages with empty
`segments`. For each `table`, the stage sends Codex one request containing a
pixel crop at the table's exact bounds, with no padding, and the
[table prompt](prompt.md). The prompt supplies all transcription instructions
and defines the response format. The stage sends no separate JSON schema.
With the transcription, it replaces the table's `md_text` and records
`table_format`.

Before saving, the stage escapes literal angle-bracket notation in the returned
markup when that notation appears in the extracted PDF text. Formatting tags
added by the transcription, such as `<br>`, remain markup when they are absent
from the PDF text.
An `unconverted` result retains the original table content.

The stage writes a full page JSON to `05_transform_page_tables/` only when the
page data changes. Pages without tables and unchanged pages remain in Stage 04
or Stage 03. The stage preserves other source objects and their IDs, geometry
and order. After execution, it removes the temporary request crops. The console
reports each table's start, format and elapsed time, then whether the coordinator
saved or skipped the page.

The configured `llm.concurrency` limits how many table requests can run at
once, even when the tables share a page. Each worker uses a separate Codex
client. The coordinator applies the results and writes a page only after the
requests for all its tables succeed.

Run through the [pipeline](../../README.md) with `--from-stage 05 --to-stage 05`
after successful Stages 02 and 04. The configured model/effort and shared response
cache apply. The status entry records this invocation's page range.

To extract graphic assets, [Stage 06](../06_extract_crops/README.md) reads
these replacement pages where available and falls back to Stages 04 and 03.
[Stage 07](../07_repair_callouts/README.md) reads the same collection of pages
to assemble callouts. [Stage 08](../08_proofreading/README.md) then proofreads
the final page objects for book import.

# Stage 05: Transform Page Tables

Read selected Stage 04 page JSONs and original Stage 02 PNGs. For each `table`,
make one Codex request using its exact, unpadded pixel crop and the
[table prompt](prompt.md). Replace table `md_text` and record `table_format`.
The prompt is the sole source of transcription instructions and response format;
no separate JSON schema is sent.
Before saving, literal angle-bracket notation present in the extracted PDF text
is escaped in the returned markup. Formatting tags added by the transcription,
such as `<br>`, remain markup when they are absent from the PDF text.
An `unconverted` result retains the original table content.

Save a full page JSON to `05_transform_page_tables/` only when its data changes.
Pages without tables and unchanged pages remain in Stage 04. Preserve other
source objects, their IDs, geometry and order. Remove temporary request crops
after execution. Console progress reports each table's start, format and elapsed
time, followed by the page's save/skip outcome.
Table requests run with the configured `llm.concurrency` limit, including tables
from the same page. Each worker owns a separate Codex client. The coordinator
applies results and writes a page only after all its tables complete successfully.

Run through the [pipeline](../../README.md) with `--from-stage 05 --to-stage 05`
after successful Stages 02 and 04. The configured model/effort and shared response
cache apply. The status entry records this invocation's page range.

[Stage 06](../06_extract_crops/README.md) overlays these replacement pages on
Stage 04 and extracts graphic assets. [Stage 07](../07_repair_callouts/README.md)
uses the same overlay to produce the final page objects for book import.

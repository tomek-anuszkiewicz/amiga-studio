# Stage 05: Transform Page Tables

Read selected Stage 04 page JSONs and original Stage 02 PNGs. For each `table`,
make one Codex request using its exact, unpadded pixel crop and the
[table prompt](prompt.md). Replace table `md_text` and record `table_format`.
Use Markdown for simple tables and HTML with `rowspan`/`colspan` for merged cells.
An `unconverted` result retains the original table content.

Save a full page JSON to `05_transform_page_tables/` only when its data changes.
Pages without tables and unchanged pages remain in Stage 04. Preserve other
source objects, their IDs, geometry and order. Remove temporary request crops
after execution. Console progress reports each table's start, format and elapsed
time, followed by the page's save/skip outcome.

Run through the [pipeline](../../README.md) with `--from-stage 05 --to-stage 05`
after successful Stages 02 and 04. The configured model/effort and shared response
cache apply. The status entry records this invocation's page range.

This is the final preparation stage. Future
[book processing](../../../book-to-markdown/README.md) combines its replacement
pages with the selected Stage 04 dataset and Stage 02 images/text.

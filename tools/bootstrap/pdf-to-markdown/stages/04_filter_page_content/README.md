# Stage 04: Filter Page Content

Read selected Stage 03 page JSONs and remove objects whose type is exactly
`header`, `footer` or `thumb_index`. Preserve every surviving object's fields, ID and order.
Covers, contents, indexes and other source roles remain for later manual editing.
No model request, image inspection or further role inference is performed.

Write retained pages to `04_filter_page_content/` using their original filenames.
Omit pages emptied by filtering; an empty result is valid. Console output reports
page/object counts and the number of emptied pages.

Run through the [pipeline](../../README.md) with `--from-stage 04 --to-stage 04`
after successful Stages 02 and 03. [Stage 05](../05_transform_page_tables/README.md)
uses only this filtered dataset; removed pages are not recovered from Stage 03.

# Stage 04: Filter Page Content

Read selected Stage 03 page JSONs and remove objects whose type is exactly
`header`, `footer` or `thumb_index`. Preserve every surviving object's fields, ID and order.
Covers, contents, indexes and other source roles remain for later manual editing.
No model request, image inspection or further role inference is performed.

Write a full page JSON to `04_filter_page_content/` only when filtering changes
its objects, using the original filename. Unchanged pages remain in Stage 03.
Save an empty `segments` list when filtering removes every object; this overrides
Stage 03 so later readers omit the page instead of restoring excluded content.
Console output reports retained page/object counts and the number of emptied pages.

Run through the [pipeline](../../README.md) with `--from-stage 04 --to-stage 04`
after successful Stages 02 and 03. [Stage 05](../05_transform_page_tables/README.md)
reads this filtered dataset with Stage 03 as the source of unchanged pages.

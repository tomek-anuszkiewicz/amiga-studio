# Stage 04: Filter Page Content

Stage 04 reads the selected Stage 03 page JSON files and removes objects whose
type is exactly `header`, `footer` or `thumb_index`. It preserves every remaining
object's fields, ID and relative order. Covers, contents, indexes and
other source roles remain for later manual editing. The stage makes no model
requests, inspects no images and infers no further roles.

Stage 04 writes the complete JSON for each changed page to
`04_filter_page_content/` using the original filename. Unchanged pages remain
in Stage 03. If Stage 04 removes every object on a page, it writes an empty
`segments` list. Later stages read that empty list in place of the Stage 03
version and omit the page. The console reports how many pages and objects
remain and how many pages the filter emptied.

Run through the [pipeline](../../README.md) with `--from-stage 04 --to-stage 04`
after successful Stages 02 and 03. [Stage 05](../05_transform_page_tables/README.md)
reads this filtered dataset with Stage 03 as the source of unchanged pages.

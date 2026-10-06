# Stage 02d: Independent Page Conversion

Read validated Stage 01 original-resolution PNGs and their complete text JSON.
The model defines logical objects independently of OCR block boundaries, returns
reading order and converts source text into `md_text` without supplied Markdown
formatting recipes. The isolated authenticated Codex request uses original image
detail and its explicit `02d_page_conversion` model/effort configuration.

Output: `02d_page_conversion/page_NNNN_segments.json`, recording `page`,
`image_width`, `image_height` and ordered `segments`. Each segment has its
post-parse deterministic `segment_id`, `type`, `heading_level`, `md_text` and `bbox`.
Only `table` and `graphic` have boxes: integer original-PNG pixels, top-left origin,
`[x0,y0,x1,y1]`, exclusive upper bounds. Textual boxes are null. No raw_text is emitted.
Schema and contextual geometry/page validation run before response caching.
Standard JSON serialization preserves decoded Markdown without pre-escaping.

Use the [orchestrator](../../README.md); do not run workers for real conversions.
Stage 02m reviews these objects. Stages 03-14 continue using the separate old branch.
The [prompt](prompt.md) defines source roles, including footnote, table_legend,
index and dedicated table/figure listing headings and entries. Source captions stay
separate from source explanations and generated asset descriptions remain deferred.

`table_legend` is source text below a table explaining symbols, notation or
abbreviations used in it. A LEGEND, NOTES or DESCRIPTION title alone does not
establish that role. General table explanations are prose; source table numbers
and titles are captions; individually referenced notes remain footnotes.
Existing artifacts with the former classification require regeneration through
02d/02m rather than relabeling their saved JSON. Stage 02k does not classify objects.

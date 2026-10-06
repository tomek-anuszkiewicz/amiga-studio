# Stage 02: Independent Page Conversion

Read validated Stage 01 original-resolution PNGs and their complete text JSON.
The model defines logical objects independently of OCR block boundaries, returns
reading order and converts source text into `md_text` without supplied Markdown
formatting recipes. The isolated authenticated Codex request uses original image
detail and its explicit `02_page_conversion` model/effort configuration.

Output: `02_page_conversion/page_NNNN_segments.json`, recording `page`,
`image_width`, `image_height` and ordered `segments`. Each segment has its
post-parse deterministic `segment_id`, `type`, `heading_level`, `md_text` and `bbox`.
Every object has a nonempty box: integer original-PNG pixels, top-left origin,
`[x0,y0,x1,y1]`, exclusive upper bounds. The model bounds complete logical objects,
including text, captions, legends and page furniture. No raw_text is emitted.
Text boxes support graphical review; later Markdown assembly may ignore them.
Table/graphic boxes can also define crops. No OCR geometry is substituted.
Schema and contextual geometry/page validation run before response caching.
Standard JSON serialization preserves decoded Markdown without pre-escaping.

Use the [orchestrator](../../README.md); do not run workers for real conversions.
Stage 02.5 reviews these objects. Stage 03 reads them directly to build the
downstream stream; review PNGs are not stream inputs.
The [prompt](prompt.md) defines source roles, including footnote, table_legend,
index and dedicated table/figure listing headings and entries. Source captions stay
separate from source explanations and generated asset descriptions remain deferred.

`cover` identifies a complete front or back book cover as one object, including
its artwork, logos and source-visible title and publication text. Its box bounds
the whole cover; `heading_level` is null and `continuation` is false. Stage 02.5
displays its label and frame through the same generic object renderer. Regenerate
02/02.5 to classify existing cover pages; saved objects are not relabeled in place.

`table_legend` is source text below a table explaining symbols, notation or
abbreviations used in it. A LEGEND, NOTES or DESCRIPTION title alone does not
establish that role. General table explanations are prose; source table numbers
and titles are captions; individually referenced notes remain footnotes.
Determine complete object boundaries before classification. A marker-referenced
footnote retains its continuation lines and paragraphs, including symbol or
abbreviation definitions. Only an independent source block is a table legend.
Existing artifacts with the former classification require regeneration through
02/02.5 rather than relabeling their saved JSON.

Explanatory text boxes connected to diagram artwork by arrows, leader lines,
shared borders or other meaningful geometry belong inside one complete `graphic`.
This grouping also covers connected marker-referenced text. Separate explanations
remain prose or footnotes according to their role; proximity and decorative borders
alone do not make text part of a graphic. Regenerate 02/02.5 to apply this grouping.

The earlier crop-only contract allowed null boxes on text. Such 02/02.5 artifacts
require regeneration for the current all-object review; no boxes are synthesized
from OCR or inserted into existing saved JSON.

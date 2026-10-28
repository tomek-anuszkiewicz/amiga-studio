# Book Element Types and Roles

Each element has an independent `type` for representation and `role` for its
semantic function. PDF [Stage 03](../pdf-to-markdown/stages/03_page_conversion/prompt.md)
chooses both in one request using the page image and extracted text. Role metadata
supports later authored grouping; it does not establish group membership.

| Type | Meaning |
| --- | --- |
| `cover` | A complete front or back book cover, including its publication text and artwork. |
| `header` | A running page header or repeated chapter/section title in the top margin. |
| `footer` | A running page footer or printed page number. |
| `toc_heading` | The title of the formal table of contents, including a continuation title; treated as a level-1 heading. |
| `toc` | Formal table-of-contents entries and their page references. |
| `thumb_index` | Printed edge tabs and marginal chapter navigation markers. |
| `heading` | A chapter, appendix, section or subsection title. Its `heading_level` property defines its depth in the document hierarchy. |
| `callout` | A source advisory label, such as NOTE, WARNING or CAUTION; after PDF Stage 07 repair, a complete Markdown callout including its body. |
| `callout_text` | The body of an advisory callout, including its paragraphs, lists or formulas. |
| `prose` | Narrative paragraphs and general explanations, including run-in titles. |
| `code_block` | Source code, assembly, pseudocode, scripts and similar program or command listings. Preserve their tokens, indentation and line breaks. |
| `preformatted_text` | Fixed-width source text whose original spacing, alignment and line breaks carry meaning: ASCII art, character-built diagrams, dumps and aligned technical layouts. Preserve it as written. |
| `table` | A table of related rows and columns, represented as Markdown or HTML after transcription. |
| `graphic` | An image, diagram, schematic, photograph or complete technical drawing composition. This is the existing identifier for an image element. |
| `index_heading` | An index title, including a continuation title; treated as a level-1 heading. |
| `index` | Alphabetical or subject-index entries and their page references. |
| `list_of_tables_heading` | A list-of-tables title, including a continuation title; treated as a level-1 heading. |
| `list_of_tables` | List-of-tables entries, including table identifiers, titles and page references. |
| `list_of_figures_heading` | A list-of-figures or illustrations title, including a continuation title; treated as a level-1 heading. |
| `list_of_figures` | List-of-figures entries, including figure identifiers, titles and page references. |

## Semantic roles

Every new element includes `role`, with one of these values:

| Role | Meaning |
| --- | --- |
| `null` | No special semantic function. |
| `caption` | A separate formal table or graphic number and title. |
| `legend` | Definitions of symbols, line styles, colors, notation or abbreviations. |
| `note` | Source explanations, assumptions, conditions or qualifications, including standalone notes. |
| `footnote` | A source note referenced by a marker, including the marker and its complete content. |

A symbol-definition table is `type: table, role: legend`; a qualification paragraph
is `type: prose, role: note`. Changing role preserves type and content. Advisory
labels and bodies remain `callout` and `callout_text`, normally with null role.
The Markdown representation and heading depth continue to follow type.
Tables with `legend`, `note` or `footnote` roles supply description context; they
are not separate description targets.

Only the current format is supported. Every element must include an explicit
role, including null when no role applies. Old semantic types and missing fields
are not translated or repaired; regenerate the data with the current converter
and importer when requested.

## Code and preformatted text

Use `code_block` for program or command listings and `preformatted_text` for material whose character layout must be preserved, such as ASCII timing diagrams, bit-field layouts, memory dumps and aligned explanatory text. A monospaced font alone does not establish that something is source code.

Render `preformatted_text` in a fenced `text` block, preserving its characters, spaces, indentation, blank lines and line breaks. Do not reflow it into prose or reconstruct it as a table or drawing. Render `code_block` with an appropriate language tag when known, preserving the source listing. Mixed objects can be explicitly split during recipe authoring; this vocabulary does not trigger automatic splitting or reclassification of saved PDF objects.

## Notes, legends and groups

An authored table or image group can associate its `table` or `graphic` with elements carrying `caption`, `note`, `legend` or `footnote` roles. Group membership identifies the source content each text block belongs to; proximity alone does not establish that relationship. A standalone `note` does not require membership in a table or image group.

Use `note` for source notes and qualifications, and `legend` for definitions needed to interpret notation. A NOTES or LEGEND label alone does not determine the role. General narrative remains `prose`, and a table's or graphic's formal name remains `caption`. Text integrated into the artwork stays inside the `graphic`; these roles describe separate source blocks, not a requirement to extract labels from an image.

Prepared groups provide the source context for the [image description prompt](prompts/describe-images.md). Generated descriptions are additions to those source elements, not source notes or legends. Table descriptions follow the [table description prompt](prompts/describe-tables.md). PDF Stage 05 continues to replace table markup only; grouping and descriptions are separate book-processing work.

A `table` or `graphic` can also carry an authored `description` string. The
context action generates it from that element, its chosen nearby caption and
contiguous notes, legends and footnotes below it, including the image asset
when applicable. A graphic or table description appears in a separate collapsed
`<details>` block containing a fenced `text` block below its source element in
rendered Working and exported Markdown; the stored description remains plain
text. The selected context is used for the task; it does not create persistent
group membership.
Descriptions are model-authored additions, not PDF source recipes. Source-text
alignment and **Clean direct book text** inspect chapter titles and `md_text`;
the cleanup action also inspects an explicitly edited generated contents title,
but never the `description` field of a table or graphic.

## Heading hierarchy

Use one `heading` type with an integer `heading_level` property for hierarchy depth. The section's source number is a separate optional `number` string; it identifies the section rather than determining its type.

| Type | Level | Example number |
| --- | --- | --- |
| `heading` | 1 | `1` |
| `heading` | 2 | `1.1` |
| `heading` | 3 | `1.1.1` |

Unnumbered titles still have a level and omit `number`. Preserve appendix identifiers such as `A` or `A.1` as strings when present. Do not introduce `chapter1`, `chapter2` or `chapter3` types; chapter and appendix openings use `heading` at the appropriate level. The PDF preparation vocabulary remains separate from this book-authoring model.

`toc_heading`, `index_heading`, `list_of_tables_heading` and `list_of_figures_heading` retain their semantic types but use `heading_level: 1`. Book processing treats them like `heading` with `heading_level: 1` for hierarchy and Markdown heading rendering (`#`). Their types identify which contents, index or list they introduce; the following entries retain their own element types.

`continuation`, `heading_level`, `number`, geometry and group membership are properties of elements rather than additional element types.

## Editable review model

Each version has its own `book.json` with ordered `chapters` and
`toc: {depth: 2}`. The generated TOC is mandatory; depth may be 1, 2 or 3.
Its optional `title` and `markdown_file` properties control the generated
heading and export filename; the defaults are `Table of Contents` and
`table-of-contents.md`. It appears separately in Rename Chapters.
A chapter has `id`, `title`, `markdown_file`
and ordered `elements`. Title and safe filename are derived from its first
heading when the chapter is created. Rename Chapters edits these fields
independently and updates the exported filename and contents links.

The importer retains provisional source classifications without automatic
grouping or reclassification. Each element has a stable `id` (the imported
`segment_id`), `type`, `role`, `visible` (default true), applicable `md_text`,
`heading_level`, `table_format`, and a graphic `asset: assets/<id>.png`.
`pdf_regions` contains ordered `{page, bbox}` records in original Stage 02 pixels.
It has no duplicated segment ID, image dimensions or OCR block associations.
Joined elements may have several regions. Splitting retains role; merging requires matching roles.


`rendered.json` records the exact completed model, chapter HTML documents,
asset hashes, input fingerprint, preparation settings and revision. Each
`render.chapters` entry contains its chapter `id`, cache `key` and bundle-relative
`document` path. Display geometry is measured from the viewer's live DOM by
stable element ID; no PNG `render_regions` or TOC hit rectangles are generated.
Authored input has no derived geometry or schema-version field. See the [review workflow](README.md#local-html-review)
for the queue and whole-version acceptance contract.

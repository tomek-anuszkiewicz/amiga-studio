# Book Element Types and Roles

An element's `type` describes how it is represented. Its independent `role`
describes what it does in the source. For each page, PDF Stage 03 uses the
page image and extracted text to choose both values in one request. Authors
use those roles later when they decide which elements to group. The author
records group membership separately. [Stage 03](../pdf-to-markdown/stages/03_page_conversion/prompt.md)

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

Every new element includes `role`. The allowed values appear below.

| Role | Meaning |
| --- | --- |
| `null` | No special semantic function. |
| `caption` | A separate formal table or graphic number and title. |
| `legend` | Definitions of symbols, line styles, colors, notation or abbreviations. |
| `note` | Source explanations, assumptions, conditions or qualifications, including standalone notes. |
| `footnote` | A source note referenced by a marker, including the marker and its complete content. |

A symbol-definition table uses `type: table, role: legend`. A paragraph that
states a qualification uses `type: prose, role: note`. Changing the role
preserves the type and content. Advisory labels and bodies use `callout` and
`callout_text`, normally with a null role. The type still governs Markdown
representation and heading depth. Tables with `legend`, `note` or `footnote`
roles provide context for another element's description. The description action
cannot target these tables separately.

PDF-to-Markdown and Book-to-Markdown support only the current format. Every
element must include an explicit role, including null when no role applies.
Regenerate the data with the current converter and importer when requested.
The current format requires current semantic types and all required fields,
with no translation or repair of old types or missing fields.

## Code and preformatted text

Use `code_block` for program or command listings. Use `preformatted_text` for
material whose character layout carries meaning, including ASCII timing diagrams,
bit-field layouts, memory dumps and aligned explanatory text. A monospaced
font alone does not establish that something is source code.

Render `preformatted_text` in a fenced `text` block. Keep its characters,
spaces, indentation, blank lines and line breaks. Do not reflow it into prose
or reconstruct it as a table or drawing. Preserve the source listing when
rendering `code_block` and use an appropriate language tag when known.
Authors can explicitly split mixed objects while writing a recipe. The
vocabulary itself does not trigger automatic splitting or reclassification of
saved PDF objects.

## Notes, legends and groups

An author can group a `table` or `graphic` with elements carrying `caption`,
`note`, `legend` or `footnote` roles. Group membership records which source
content each text block belongs to. Nearby placement alone cannot establish
that relationship. A standalone `note` can remain outside any table or image
group.

Use `note` for source notes and qualifications, and `legend` for definitions needed to interpret notation. A NOTES or LEGEND label alone does not determine the role. General narrative
remains `prose`, and a table's or graphic's formal name remains `caption`.
These roles apply to separate source blocks. Leave text that is integrated
into the artwork inside the `graphic`.

The image and table description tasks use prepared groups as source context.
They add generated descriptions alongside the source notes and legends.
Grouping and description are book-processing steps. PDF Stage 05 replaces
only table markup. [Image description prompt](prompts/describe-images.md)
[Table description prompt](prompts/describe-tables.md)

A `table` or `graphic` can also carry an authored `description` string. The
context action generates the selected element's description from the element
itself, its chosen nearby caption, contiguous notes, legends and footnotes
below it, and the image asset when applicable.
It uses that selection for the task only, without recording group membership.
The editor stores the description as plain text. In rendered Working and
exported Markdown, it appears below the element in a separate collapsed
`<details>` block containing a fenced `text` block.
Descriptions store text the model adds to explain a table or graphic.
The PDF source recipe records how to reconstruct content from the source.
Source-text alignment and
**Clean direct book text** inspect chapter titles and `md_text`. The cleanup
action also inspects the generated contents title when explicitly edited.
It skips the `description` field of every table or graphic.

## Heading hierarchy

Use one `heading` type and an integer `heading_level` property to record
hierarchy depth. When present, the source number identifies the section and
goes in the separate optional `number` string. The element type stays the
same at every numbered level.

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
`toc: {depth: 2}`. The generated table of contents is required and can have a
depth of 1, 2 or 3. Its optional `title` and `markdown_file` properties control
the generated heading and export filename. The defaults are `Table of Contents`
and `table-of-contents.md`. The table of contents has its own entry in Rename
Chapters.
A chapter has `id`, `title`, `markdown_file` and ordered `elements`. When the
editor creates a chapter, it derives the title and a safe filename from the
first heading. Rename Chapters lets you edit the title and filename independently
and updates the exported filename and contents links.

The importer retains provisional source classifications without automatic
grouping or reclassification. Each element has a stable `id` (the imported
`segment_id`), `type`, `role`, `visible` (default true), applicable `md_text`,
`heading_level`, `table_format`, and a graphic `asset: assets/<id>.png`.
`pdf_regions` contains ordered `{page, bbox}` records in original Stage 02 pixels.
It has no duplicated segment ID, image dimensions or OCR block associations.
Joined elements may have several regions. Splitting an element preserves its
role. To merge elements, their roles must match.

`rendered.json` records the exact completed model, chapter HTML documents,
asset hashes, input fingerprint, preparation settings and revision. Each
`render.chapters` entry contains its chapter `id`, cache `key` and bundle-relative
`document` path. Display geometry comes from the viewer's live DOM and is
measured by stable element ID. No PNG `render_regions` or TOC hit rectangles
are generated.
Authored input has no derived geometry or schema-version field. See the [review workflow](README.md#local-html-review)
for the queue and whole-version acceptance contract.

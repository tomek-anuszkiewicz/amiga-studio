# Book Element Types

This initial vocabulary describes the source elements distinguished during book recipe authoring. It starts from the [Stage 02 classification prompt](../pdf-to-markdown/stages/02_page_conversion/prompt.md) and uses generic `note` and `legend` roles. Their association with a table, image or passage belongs to authored grouping. It is a book-authoring draft; the PDF Stage 02 response schema remains unchanged.

| Type | Meaning |
| --- | --- |
| `cover` | A complete front or back book cover, including its publication text and artwork. |
| `header` | A running page header or repeated chapter/section title in the top margin. |
| `footer` | A running page footer or printed page number. |
| `toc_heading` | The title of the formal table of contents, including a continuation title; treated as a level-1 heading. |
| `toc` | Formal table-of-contents entries and their page references. |
| `thumb_index` | Printed edge tabs and marginal chapter navigation markers. |
| `heading` | A chapter, appendix, section or subsection title. Its `level` property defines its depth in the document hierarchy. |
| `callout` | A standalone advisory label, such as NOTE, WARNING or CAUTION. |
| `callout_text` | The body of an advisory callout, including its paragraphs, lists or formulas. |
| `prose` | Narrative paragraphs and general explanations, including run-in titles. |
| `code_block` | Source code, assembly, pseudocode, scripts and similar program or command listings. Preserve their tokens, indentation and line breaks. |
| `preformatted_text` | Fixed-width source text whose original spacing, alignment and line breaks carry meaning: ASCII art, character-built diagrams, dumps and aligned technical layouts. Preserve it as written. |
| `table` | A table of related rows and columns, represented as Markdown or HTML after transcription. |
| `graphic` | An image, diagram, schematic, photograph or complete technical drawing composition. This is the existing identifier for an image element. |
| `caption` | A separate formal table or figure number and title. |
| `footnote` | A source note referenced by a marker; preserve the marker and the complete note. |
| `note` | A source note containing explanations, assumptions, conditions or qualifications. It can belong to a table or image group, or stand independently in the text. Keep its introductory label with the note. |
| `legend` | A separate block defining symbols, line styles, colors, notation or abbreviations. Its association with the relevant source content belongs to grouping. |
| `index_heading` | An index title, including a continuation title; treated as a level-1 heading. |
| `index` | Alphabetical or subject-index entries and their page references. |
| `list_of_tables_heading` | A list-of-tables title, including a continuation title; treated as a level-1 heading. |
| `list_of_tables` | List-of-tables entries, including table identifiers, titles and page references. |
| `list_of_figures_heading` | A list-of-figures or illustrations title, including a continuation title; treated as a level-1 heading. |
| `list_of_figures` | List-of-figures entries, including figure identifiers, titles and page references. |

## Code and preformatted text

Use `code_block` for program or command listings and `preformatted_text` for material whose character layout must be preserved, such as ASCII timing diagrams, bit-field layouts, memory dumps and aligned explanatory text. A monospaced font alone does not establish that something is source code.

Render `preformatted_text` in a fenced `text` block, preserving its characters, spaces, indentation, blank lines and line breaks. Do not reflow it into prose or reconstruct it as a table or drawing. Render `code_block` with an appropriate language tag when known, preserving the source listing. Mixed objects can be explicitly split during recipe authoring; this vocabulary does not trigger automatic splitting or reclassification of saved PDF objects.

## Notes, legends and groups

An authored table or image group can associate its `table` or `graphic` with a `caption`, `note`, `legend` and any relevant `footnote` objects. Group membership identifies the source content each text block belongs to; proximity alone does not establish that relationship. A standalone `note` does not require membership in a table or image group.

Use `note` for source notes and qualifications, and `legend` for definitions needed to interpret notation. A NOTES or LEGEND label alone does not determine the role. General narrative remains `prose`, and a table's or figure's formal name remains `caption`. Text integrated into the artwork stays inside the `graphic`; these roles describe separate source blocks, not a requirement to extract labels from an image.

Prepared groups provide the source context for the [image description prompt](prompts/describe-images.md). Generated descriptions are additions to those source elements, not source notes or legends. Table descriptions follow the [table description prompt](prompts/describe-tables.md). PDF Stage 02.81 continues to replace table markup only; grouping and descriptions are separate book-processing work.

## Heading hierarchy

Use one `heading` type with an integer `level` property for hierarchy depth. The section's source number is a separate optional `number` string; it identifies the section rather than determining its type.

| Type | Level | Example number |
| --- | --- | --- |
| `heading` | 1 | `1` |
| `heading` | 2 | `1.1` |
| `heading` | 3 | `1.1.1` |

Unnumbered titles still have a level and omit `number`. Preserve appendix identifiers such as `A` or `A.1` as strings when present. Do not introduce `chapter1`, `chapter2` or `chapter3` types; chapter and appendix openings use `heading` at the appropriate level. The PDF preparation vocabulary remains separate from this book-authoring model.

`toc_heading`, `index_heading`, `list_of_tables_heading` and `list_of_figures_heading` retain their semantic types but use `level: 1`. Book processing treats them like `heading` with `level: 1` for hierarchy and Markdown heading rendering (`#`). Their types identify which contents, index or list they introduce; the following entries retain their own element types.

`continuation`, `level`, `number`, geometry and group membership are properties of elements rather than additional element types.

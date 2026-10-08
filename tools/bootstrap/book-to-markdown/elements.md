# Book Element Types

This initial vocabulary describes the source elements distinguished during book recipe authoring. It starts from the [Stage 02 classification prompt](../pdf-to-markdown/stages/02_page_conversion/prompt.md) and adds `image_note` and `image_legend` for explicitly identified image groups. It is a book-authoring draft; the PDF Stage 02 response schema remains unchanged.

| Type | Meaning |
| --- | --- |
| `cover` | A complete front or back book cover, including its publication text and artwork. |
| `header` | A running page header or repeated chapter/section title in the top margin. |
| `footer` | A running page footer or printed page number. |
| `toc_heading` | The title of the formal table of contents, including a continuation title. |
| `toc` | Formal table-of-contents entries and their page references. |
| `thumb_index` | Printed edge tabs and marginal chapter navigation markers. |
| `chapter` | A chapter or appendix opening. |
| `heading` | A section or subsection title establishing document hierarchy. |
| `callout` | A standalone advisory label, such as NOTE, WARNING or CAUTION. |
| `callout_text` | The body of an advisory callout, including its paragraphs, lists or formulas. |
| `prose` | Narrative paragraphs and general explanations, including run-in titles. |
| `code_block` | Code, assembly, dumps, aligned technical text and character-built diagrams whose spacing carries meaning. |
| `table` | A table of related rows and columns, represented as Markdown or HTML after transcription. |
| `graphic` | An image, diagram, schematic, photograph or complete technical drawing composition. This is the existing identifier for an image element. |
| `caption` | A separate formal table or figure number and title. |
| `footnote` | A source note referenced by a marker; preserve the marker and the complete note. |
| `table_legend` | A separate block defining a table's symbols, notation or abbreviations. |
| `image_note` | A separate source note attached to a specific image: explanations, assumptions, conditions or qualifications. Keep its introductory label with the note. |
| `image_legend` | A separate block defining an image's symbols, line styles, colors, notation or abbreviations. |
| `index_heading` | An index title, including a continuation title. |
| `index` | Alphabetical or subject-index entries and their page references. |
| `list_of_tables_heading` | A list-of-tables title, including a continuation title. |
| `list_of_tables` | List-of-tables entries, including table identifiers, titles and page references. |
| `list_of_figures_heading` | A list-of-figures or illustrations title, including a continuation title. |
| `list_of_figures` | List-of-figures entries, including figure identifiers, titles and page references. |

## Image groups

An authored image group can associate its `graphic` with a `caption`, `image_note`, `image_legend` and any relevant `footnote` objects. Group membership identifies the image each text block belongs to; proximity alone does not establish that relationship.

Use `image_note` for explanations and qualifications, and `image_legend` for definitions needed to interpret visual notation. A NOTES or LEGEND label alone does not determine the role. General narrative remains `prose`, and a figure's formal name remains `caption`. Text integrated into the artwork stays inside the `graphic`; these additional types describe separate source blocks, not a requirement to extract labels from an image.

Prepared groups provide the source context for the [image description prompt](prompts/describe-images.md). Generated descriptions are additions to those source elements, not source notes or legends. Table descriptions follow the [table description prompt](prompts/describe-tables.md). PDF Stage 02.81 continues to replace table markup only; grouping and descriptions are separate book-processing work.

`continuation`, heading level, geometry and group membership are properties of elements rather than additional element types.

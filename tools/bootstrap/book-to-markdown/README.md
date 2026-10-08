# Book-to-Markdown

This directory starts the planned deterministic chapter generator. It will consume prepared data from [PDF-to-Markdown](../pdf-to-markdown/README.md) and authored JSON recipes. The generator is not implemented yet.

PDF preparation ends at Stage 02.81. The intended handoff uses Stage 01 original
page PNGs and positioned text, Stage 02.80 filtered page objects and Stage 02.81
changed page objects. The 02.81 status entry records its selected page range;
overlay replacements on those selected 02.80 pages and retain unchanged pages.
The generator will emit chapter Markdown and crop graphic assets directly from
this prepared data. No intermediate page-ordered Markdown export is required.

## Element vocabulary

The initial [element list](elements.md) records source roles used in book authoring. Notes and legends use generic `note` and `legend` types; authored groups associate them with tables or images, and notes may also stand independently in the text. Image elements retain the existing `graphic` identifier. Grouping is authored separately from PDF table transcription.

Chapter, appendix and section titles use `heading` with a `level` property for hierarchy depth and an optional `number` string for their source numbering.
The dedicated contents, index, table-list and figure-list heading types retain their roles and are treated as level-1 headings.

`code_block` represents source code and similar listings. `preformatted_text` preserves original character layout for ASCII art, dumps and other aligned text, rendered in a fenced `text` block.

## Chapter proofreading

The initial [proofreading instruction](prompts/proofreading.md) is intended for manual agent processing of an already generated Markdown chapter. Supply the instruction and chapter; the agent can inspect linked images when needed. Keep the generated source chapter available for comparison and retain the edited copy outside generator-managed output cleanup.

The instruction combines source-faithful OCR correction with limited Markdown formatting, including inline-code emphasis of technical terms. It preserves table structure, code/ASCII alignment, chapter organization and links. HTML blocks use HTML formatting and escaping rather than Markdown inline syntax. Image/table descriptions remain separately requested tasks.

This draft preserves chapter Markdown structure and filenames during explicitly
requested proofreading and formatting. Its prompts are self-contained.
Further scope reduction follows user review.

## Table and image descriptions

Two initial prompts support separately requested descriptions:

- [Table description](prompts/describe-tables.md) explains table entries, values and relationships, including merged HTML cells.
- [Image description](prompts/describe-images.md) explains the inspected image's components, labels and visual relationships.

Supply one already identified group in a `<details>` section: its name or caption, associated notes and legend, and the Markdown/HTML table or image reference. The prompts use this grouping directly. They return an English Markdown description for insertion into the same section while preserving the source material. Image processing requires access to the actual image. These are prompt drafts for manual agent use; automatic execution and insertion are not implemented.

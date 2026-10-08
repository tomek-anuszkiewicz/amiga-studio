# Book-to-Markdown

This directory starts the planned deterministic chapter generator. It will consume prepared data from [PDF-to-Markdown](../pdf-to-markdown/README.md) and authored JSON recipes. The generator is not implemented yet.

PDF preparation ends at Stage 05. The intended handoff uses Stage 02 original
page PNGs and positioned text, Stage 04 filtered page objects and Stage 05
changed page objects. The 05 status entry records its selected page range;
overlay replacements on those selected 04 pages and retain unchanged pages.
The generator will emit chapter Markdown and crop graphic assets directly from
this prepared data. No intermediate page-ordered Markdown export is required.

## Preparing source PDFs (author workflow)

The project author prepares the local PDF collection while authoring book
recipes. This is a development prerequisite, not a step in the ordinary user's
chapter-generation workflow.

The current source catalog uses these book directory names:

| PDF to obtain | Directory under `books/` |
| --- | --- |
| Amiga Hardware Reference Manual, second edition (1989) | `amiga-hardware-reference-manual-2nd-edition` |
| Amiga A500/A2000 Technical Reference Manual | `amiga-a500-a2000-technical-reference-manual` |
| M68000 Programmer's Reference Manual | `m68000-programmers-reference-manual` |
| M68000 User's Manual, Revision 8 | `m68000-users-manual-rev8` |

For each required source variant:

1. Find the PDF online and download the required manual and edition locally.
   Its download filename and hosting location may differ between mirrors.
2. Calculate the downloaded file's full SHA-256, for example with PowerShell:
   `Get-FileHash -LiteralPath './downloaded.pdf' -Algorithm SHA256`.
3. Compare all 64 hexadecimal characters with the `sha256` value in the
   variant's `source.json`. A matching title, page count or 12-character prefix
   alone does not identify the accepted PDF. If the hash differs, obtain the
   matching source or deliberately author a separate variant; do not replace
   an existing variant's hash to accept another download.
4. Copy the matching PDF into that variant's directory as `source.pdf`:
   `books/<book-id>/<first-12-characters-of-sha256>/source.pdf`.
   Verify the copied file's full SHA-256 again against `source.json`.

Book IDs use lowercase names separated by hyphens. The hash suffix identifies
the exact PDF variant, and `source.json` stores its full hash. Source PDFs remain
local and are ignored by Git; the catalog metadata is tracked.

The composite Test Book lives separately in `books/test-book-example-4567/`
and does not need `source.json`. Its
[page tracker](books/test-book-example-4567/test_pages_tracker.md) records the
four source variants, their full hashes and the 160-page assembly order. Use
that mapping to reconstruct the test PDF locally after obtaining those sources.

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

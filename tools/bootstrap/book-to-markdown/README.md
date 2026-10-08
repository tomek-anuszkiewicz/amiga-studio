# Book-to-Markdown

This directory starts the planned deterministic chapter generator. It will consume prepared data from [PDF-to-Markdown](../pdf-to-markdown/README.md) and authored JSON recipes. The generator is not implemented yet.

PDF preparation ends at Stage 05. The intended handoff uses Stage 02 original
page PNGs and positioned text, Stage 04 filtered page objects and Stage 05
changed page objects. The 05 status entry records its selected page range;
overlay replacements on those selected 04 pages and retain unchanged pages.
The generator will emit chapter Markdown and crop graphic assets directly from
this prepared data. No intermediate page-ordered Markdown export is required.

## Source catalog and variant layout

The [bootstrap instructions](../AGENTS.md#preparing-the-book-source-collection)
describe source acquisition during book authoring.

The current source catalog uses these book directory names:

| PDF to obtain | Directory under `books/` |
| --- | --- |
| Amiga Hardware Reference Manual, second edition (1989) | `amiga-hardware-reference-manual-2nd-edition` |
| Amiga A500/A2000 Technical Reference Manual | `amiga-a500-a2000-technical-reference-manual` |
| M68000 Programmer's Reference Manual | `m68000-programmers-reference-manual` |
| M68000 User's Manual, Revision 8 | `m68000-users-manual-rev8` |

Book IDs use lowercase names separated by hyphens. The hash suffix identifies
the exact PDF variant, and `source.json` stores its full hash. Source PDFs remain
local and are ignored by Git; the catalog metadata is tracked.

The same convention applies to every book, including the composite Test Book:

```text
books/<book-id>/<first-12-characters-of-sha256>/
  source.pdf          # local original PDF
  source.json         # tracked full SHA-256
  pdf-to-markdown/    # local default preparation workspace, created on first run
```

The Test Book variant is `books/test-book-example-4567/1cea0b35ee62/`, with full
SHA-256 `1cea0b35ee622aec0e4c728a170edb6b9324a1baab1f14fe8c67a7f7f2fc45c0`.
Its [page tracker](books/test-book-example-4567/1cea0b35ee62/test_pages_tracker.md)
lives beside `source.pdf` and records the four source variants, their full hashes
and the 160-page assembly order. The prefix names the variant directory;
the full hash identifies the exact source PDF. Future book preparation uses this layout.

## Element vocabulary

The initial [element list](elements.md) records source roles used in book authoring. Notes and legends use generic `note` and `legend` types; authored groups associate them with tables or images, and notes may also stand independently in the text. Image elements retain the existing `graphic` identifier. Grouping is authored separately from PDF table transcription.

Chapter, appendix and section titles use `heading` with a `level` property for hierarchy depth and an optional `number` string for their source numbering.
The dedicated contents, index, table-list and figure-list heading types retain their roles and are treated as level-1 headings.

`code_block` represents source code and similar listings. `preformatted_text` preserves original character layout for ASCII art, dumps and other aligned text, rendered in a fenced `text` block.

## Chapter proofreading

The initial [proofreading instruction](prompts/proofreading.md) is intended for manual agent processing of an already generated Markdown chapter. Handling guidance is in the [bootstrap instructions](../AGENTS.md#manual-chapter-processing).

The instruction combines source-faithful OCR correction with limited Markdown formatting, including inline-code emphasis of technical terms. It preserves table structure, code/ASCII alignment, chapter organization and links. HTML blocks use HTML formatting and escaping rather than Markdown inline syntax. Image/table descriptions remain separately requested tasks.

The prompt is self-contained and preserves chapter Markdown structure and filenames.

## Table and image descriptions

Two initial prompts support separately requested descriptions:

- [Table description](prompts/describe-tables.md) explains table entries, values and relationships, including merged HTML cells.
- [Image description](prompts/describe-images.md) explains the inspected image's components, labels and visual relationships.

The prompts return an English Markdown description for insertion into the source group's `<details>` section while preserving the source material. These are prompt drafts for manual agent use; automatic execution and insertion are not implemented. Input preparation and image inspection guidance is in the [bootstrap instructions](../AGENTS.md#manual-chapter-processing).

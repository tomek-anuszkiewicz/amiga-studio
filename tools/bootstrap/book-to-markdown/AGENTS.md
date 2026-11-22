# Book recipes and reconstruction

Use this contract and the applicable bootstrap instructions when authoring a
recipe, importing a book or reconstructing it. Apply the source and editorial
requirements before you change a recipe. They also apply during review.
[Bootstrap instructions](../AGENTS.md)

## Authoritative model

- The PDF supplies text and graphics. The reconstruction recipe, `book.json`,
  selects and orders source regions, assigns chapters and roles, and records
  visibility, Markdown/HTML structure and explicit corrections.
- For source-derived `md_text`, chapter titles and filenames, refer to the
  physical PDF page, extracted block and Unicode character range. Put formatting
  around those references. Do not substitute opaque Stage 04/05 converted-Markdown
  segment references.
- Conversion results can help when you author the recipe. A finished recipe
  supports text reconstruction independently of those results. Tables keep
  their row and cell structure and spans, with references for the text.
  Reconstruction also recovers graphics from PDF regions. Generated assets
  serve local review.
- The publication target is reviewed recipes that identify their sources. With a
  matching PDF, reconstruction can rebuild Markdown without repeating model
  inference. Keep acquisition, identity and extraction explicit. Restore the
  editor with `book_review.py import-book` under the
  [roadmap](../../../ROADMAP.md#step-11-four-manual-pdf-to-book-json-conversion)
  and [recipe contract](README.md#editing-and-publication).
- The explicit Codex editor command takes the current authored text, including
  literals, as its trusted baseline. Before replacing one Working element, check
  the returned content against the exact request snapshot. Preserve the element's
  ID, source regions, role and visibility. Keep the strict source audit optional
  for responses, imports and manual edits.
- A general Codex Working task may request context and edit multiple elements
  under the user's instruction. After each tool action, validate the current
  Working revision and recipe. Accepted and source PDF data are read-only for
  this task. Only the element transformation tool uses the one-element content
  guard.
- Table-to-graphic displays original PDF crops and preserves the text. For a
  graphic without stored text, use positioned Stage 02 OCR. If OCR is absent,
  report that absence and leave the image text unspecified. Changed or stale
  model responses must never be saved.

## Scope

The parent contract governs current-format-only behavior, generated artifacts,
import timing and the boundaries of technical testing and editorial work. The
editor still supports its ordinary actions and current-format validation.
[Converter-development contract](../AGENTS.md#converter-development)

Publish, stage or commit recipes or source artifacts only at the requested time
and under the repository commit policy. The intended publication workflow alone
authorizes none of those steps.

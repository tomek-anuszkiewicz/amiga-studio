# Book Recipes and Reconstruction

These instructions supplement the [bootstrap instructions](../AGENTS.md).

## Authoritative model

- The PDF supplies source text and graphics. `book.json` is the reconstruction
  recipe: it selects and orders source regions, assigns chapters and element
  roles, and records visibility, Markdown/HTML structure and explicit corrections.
- Source-derived text in `md_text`, chapter titles and filenames must use physical
  PDF page, extracted block and Unicode character-range references. Keep Markdown
  formatting around those references. Do not replace a source recipe with an
  opaque reference to a Stage 04/05 converted Markdown segment.
- Use conversion results to author the recipe, not as a required text source for
  the finished recipe. Tables retain their row/cell structure and spans while
  their source text is referenced. Graphics are reconstructed from their PDF
  regions; generated asset files are local review artifacts.
- The publication target is reviewed recipes with source identification so a
  matching PDF can rebuild Markdown without repeating model inference. PDF
  acquisition, identity and extraction conventions must be explicit. Restore
  the editor from a matching source with `book_review.py import-book`; follow the
  [roadmap](../../../ROADMAP.md#step-11-four-manual-pdf-to-book-json-conversion)
  and [recipe contract](README.md#editing-and-publication).
- The explicit Codex editor command treats current authored text (including
  existing literals) as its trusted baseline. Validate returned content against
  the exact request snapshot before replacing one Working element; preserve its
  ID, source regions, role and visibility. Do not impose the optional strict
  source audit on model responses, imports or manual editor changes.
- The general Codex Working task may request book context and make changes across
  elements. Its user instruction governs intentional text edits; validate the
  current Working revision and recipe after each tool action. It cannot write
  Accepted or source PDF data. The one-element content guard remains specific to
  the element transformation tool.
- Table-to-graphic displays original PDF crops and retains its text. A graphic
  without stored text uses positioned Stage 02 OCR; absent OCR must be reported,
  not invented from the image. Changed or stale model responses must not save.

## Scope

The [parent bootstrap rules](../AGENTS.md#converter-development) govern
current-format-only changes, generated artifacts and import timing. Ordinary
editor operations and current-format validation remain supported.

Do not publish, stage or commit book recipes or source artifacts merely because
publication is the intended final workflow. Follow the user's requested timing
and the repository commit policy.

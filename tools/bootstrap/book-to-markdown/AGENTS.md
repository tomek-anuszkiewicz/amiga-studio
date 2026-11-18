# Book recipes and reconstruction

Load this contract for recipe authoring, import and reconstruction alongside the
applicable [bootstrap instructions](../AGENTS.md). Source/editorial requirements
apply before changing a recipe, not merely at review.

## Authoritative model

- The PDF supplies text and graphics. `book.json` selects/orders source regions,
  assigns chapters/roles and records visibility, Markdown/HTML structure and explicit
  corrections. It is the reconstruction recipe.
- Source-derived `md_text`, chapter titles and filenames use physical PDF page,
  extracted block and Unicode character-range references, with formatting around
  them. Do not substitute opaque Stage 04/05 converted-Markdown segment references.
- Conversion results help author recipes but are not required text sources for a
  finished recipe. Tables retain row/cell structure and spans with referenced text;
  graphics reconstruct from PDF regions. Generated assets are local review artifacts.
- The publication target is reviewed recipes with source identification, allowing a
  matching PDF to rebuild Markdown without repeated model inference. Keep acquisition,
  identity and extraction explicit. Restore the editor with `book_review.py import-book`
  under the [roadmap](../../../ROADMAP.md#step-11-four-manual-pdf-to-book-json-conversion)
  and [recipe contract](README.md#editing-and-publication).
- The explicit Codex editor command trusts current authored text, including literals,
  as its baseline. Check returned content against the exact request snapshot before
  replacing one Working element; preserve ID, source regions, role and visibility.
  Do not impose the optional strict source audit on responses, imports or manual edits.
- The general Codex Working task may request context and edit across elements under
  its user's instruction. Validate current Working revision and recipe after every
  tool action. It cannot write Accepted or source PDF data. The one-element content
  guard remains specific to the element transformation tool.
- Table-to-graphic shows original PDF crops and retains text. A graphic without
  stored text uses positioned Stage 02 OCR; report absent OCR rather than inventing
  text from its image. Changed or stale model responses must not save.

## Scope

The parent [converter-development contract](../AGENTS.md#converter-development)
owns current-format-only behavior, generated artifacts, import timing and technical
test/editorial boundaries. Ordinary editor actions and current-format validation
remain supported. Do not publish, stage or commit recipes/source artifacts merely
because publication is the intended eventual workflow; follow the requested timing
and repository commit policy.

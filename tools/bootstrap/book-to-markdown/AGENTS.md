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
- The publication target is a repository containing reviewed recipes and source
  identification, allowing another reader to obtain the matching PDFs and rebuild
  useful Markdown without repeating model inference. PDF acquisition, source
  identity, OCR/extraction conventions and reconstruction must be explicit.
  Do not claim that a standalone bootstrap command exists until implemented.
- Recipe changes requested by the user mean changes to the generating and
  consuming code. Do not hand-edit generated `book.json` to conceal an importer
  or renderer defect. Regeneration is a separate operation, performed when requested;
  existing authored choices and generated bundles do not constrain that operation.
- Keep implemented reconstruction separate from pending work. See the
  [documentation and RAG roadmap](../../../ROADMAP.md#step-1-manual-conversion-and-initial-knowledge-base) and [template contract](README.md#editing-and-publication).

## Scope

Use only the current recipe format. Do not implement backward compatibility,
automatic migrations, legacy interpretation, manual patches or inference-based
rescue of existing content. Import rebuilds its output and may overwrite an
existing Working book; do not preserve old content or settings to avoid loss.
Repair the generating code or prompt instead. Run import/regeneration only when
the user requests it. Ordinary editor operations and current-format validation
remain supported.

Do not publish, stage or commit book recipes or source artifacts merely because
publication is the intended final workflow. Follow the user's requested timing
and the repository commit policy.

# PDF-MARKDOWN-02.9: Markdown and Crop Assets from Stage 02

Status: planned; implementation and fragment conversion have not started.

## Goal

Add Stage `02.9_emit_page_markdown` to `tools/bootstrap/pdf-to-markdown`.
Assemble the existing Stage 02 conversion into one Markdown file for all pages
selected in the attempt, accompanied by an `assets/` directory containing image
crops. This provides a directly readable result before the later stream pipeline.

Use the existing [Stage 02 object contract](../../tools/bootstrap/pdf-to-markdown/stages/02_page_conversion/README.md)
and [conversion workflow](../../tools/bootstrap/reference-conversion-contract.md#development-workflow).
Stage 02 already supplies textual objects as `md_text`; tables, graphics and
covers have empty `md_text` and complete pixel rectangles. Stage 2.9 consumes
these rectangles as crop instructions rather than requesting new conversion.

## Input and output contract

Inputs are validated Stage 02 `page_NNNN_segments.json` files and their exact
Stage 01 original-resolution page PNGs for the workspace's selected pages.
Use existing manifest, page-pair and completion validation. Review PNGs from
Stage 02.5 are not crop sources or required inputs. Do not fall back to OCR
blocks, the source PDF, unrelated attempts or incompatible saved objects.

The output bundle is:

```text
<WORKSPACE>/02.9_emit_page_markdown/
    document.md
    assets/
        <segment_id>.png
```

Exactly one Markdown file contains all selected pages, including disjoint page
ranges in source order. Keep any orchestration/completion metadata in the
existing workspace state rather than adding per-page Markdown or asset sidecars.
Create the assets directory even when the selection contains no crop objects.

## Assembly rules

- Preserve physical page order and each page's Stage 02 segment-array order.
  Do not infer a different reading order from coordinates or object types.
- Append each textual object's decoded `md_text` without rewriting, proofreading,
  de-hyphenation or escape processing. Use blank lines between objects while
  preserving internal code, table, math and line-break formatting.
- For each `table`, `graphic` or `cover`, crop the corresponding original PNG
  using the validated integer `[x0, y0, x1, y1]` rectangle, top-left origin and
  exclusive upper bounds. Use the rectangle exactly, without padding, rescaling,
  OCR-derived bounds or automatic geometry correction.
- Save each crop as PNG using the existing deterministic page-scoped
  `segment_id` as its filename. Embed it at that object's position with a
  relative Markdown image link, for example
  `![Table p0005_s003](assets/p0005_s003.png)`; use the actual upstream ID.
  A neutral type/ID label must not invent a caption or image description.
- Keep captions, table legends, footnotes, headers, footers and other textual
  objects in their existing source order. Do not silently filter source content
  or duplicate text already contained in a complete crop.
- Keep multi-page objects as the separate crops represented by Stage 02.
  Preserve existing continuation wording; geometric stitching, continuation
  inference and cross-page text merging belong to separate work.
- Add minimal valid YAML frontmatter and one document heading using known
  source metadata, with the source stem as the title fallback. Do not invent
  publication details or regenerate a table of contents. Source-visible TOC
  text remains unchanged; link repair remains later-stage work.

This stage is deterministic and makes no model requests. Tables and illustrations
remain raster crops in this export; reconstruction and descriptions remain
separate downstream tasks.

## Execution steps

1. Add `stages/02.9_emit_page_markdown/` with a worker and README. Reuse the
   existing Stage 02 schema/page validation, Stage 01 artifact access and PNG
   tooling; avoid introducing a second object schema or coordinate convention.
2. Register CLI stage token `02.9` and deterministic stage configuration in the
   orchestrator. Place it after `02.5` and before `03` in execution order, with
   artifact dependencies on `01` and `02`. It must run alone from validated
   predecessors via `--from-stage 02.9 --to-stage 02.9`.
3. Implement ordered text/image assembly. Validate every selected input and
   crop before publishing the completed bundle. Missing/invalid boxes, page
   mismatches, duplicate asset identities or failed image writes must stop the
   stage with page/segment context rather than omit an object.
4. Write into a temporary stage-owned bundle, verify every emitted asset link
   resolves, then publish the bundle and record completion. Remove stale assets
   when replacing a prior bundle; an interrupted run must not be marked complete.
5. Bind completion/resume identity to both predecessor records, the stage
   procedure and applicable configuration. Restart `01` or `02` invalidates
   this output. Restart `02.5` retains it. Restart `02.9` invalidates only its
   own output; later stream stages retain their existing Stage 02 dependency.
6. Document the new output and standalone interval in the converter README and
   shared conversion contract. Stage 03 keeps its direct Stage 02 input.
   Existing Stage 14 output and publication routing remain separate; exporting
   this bundle through `--publish` is outside this task.

## Verification and acceptance

- Run existing technical checks relevant to the modified orchestration,
  configuration and lineage. Add a test only for a concrete technical defect;
  do not restore removed crop tests or add conversion-quality scoring.
- Run `python tools/harness/pre_flight.py --quick` and
  `cargo test -p test_runner --test test_architecture_rules -- --quiet` before
  the implementation commit. Record actual failures and unrun checks.
- When the user requests a conversion, use only the user-selected fragment in
  a named attempt workspace. Run through `02.9` and report `document.md` and
  `assets/` locations. Do not independently launch a full-book run.
- The runtime must establish one Markdown file, preserved selected-page/object
  order, successful crop writes and resolving relative asset links. Repeating
  assembly from the same validated inputs must produce the same bundle without
  model calls or accumulated stale assets.
- The user assesses whether text, reading order and crop boundaries faithfully
  reflect the selected source. Technical completion does not certify quality.

Retain this plan while implementation is pending. Close it under the repository's
execution-plan lifecycle after verified delivery or explicit user-directed closure.

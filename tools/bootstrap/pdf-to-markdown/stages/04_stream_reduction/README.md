# Stage 04: Stream Reduction

## Objective
Performs sequential normalization across the global node stream:
1. Identifies and unifies contiguous `graphic` fragments on the same page into a single consolidated diagram asset using Codex Vision (`prompt_graphics_union.md`), purging individual obsolete asset crops from `assets/` and replacing the constituent nodes with a single unified node.
2. Fuses adjacent narrative `prose` nodes across page boundaries, performing de-hyphenation on split words via LLM evaluation (`prompt_seam.md`).
3. Preserves structural node boundaries for tables, code listings, and headings.

[Stage 02.8](../02.8_filter_page_content/README.md) owns source-content filtering
before stream assembly, including header/footer removal. Stage 04 performs no
additional source-content exclusion.

Converted Stage 02.81 Markdown/HTML tables remain separate and ordered, with
their saved group text and source-image fields intact. Contiguous table unification
applies only to unconverted legacy fragments.

## Inputs
- `workspace/03_build_raw_stream/raw_stream.json`: Master sequential node stream from Stage 03.
- `workspace/01_preprocess/page_XXXX.png`: 300 DPI raster page images for vision verification.
- `workspace/01_preprocess/page_XXXX.json`: Page dimensions used with the source PNG when cropping merged fragments.
- `stages/04_stream_reduction/prompt_seam.md`: LLM prompt for cross-page text seams.
- `stages/04_stream_reduction/prompt_graphics_union.md`: Vision prompt for contiguous graphic union validation.

## Outputs
- `workspace/04_stream_reduction/reduced_stream.json`: Normalized, welded node stream.
- `workspace/04_stream_reduction/assets/`: Consolidated PNG visual assets; raw text remains in the node JSON. Merged crops come directly from Stage 01 PNGs without rereading the source PDF.

## Invocation Through the Orchestrator
```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 04 --to-stage 04
```

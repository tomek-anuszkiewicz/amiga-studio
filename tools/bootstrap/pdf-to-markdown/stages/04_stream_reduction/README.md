# Stage 04: Stream Reduction

## Objective
Performs sequential normalization across the global node stream:
1. Strips repetitive running `header` and `footer` blocks.
2. Identifies and unifies contiguous `graphic` fragments on the same page into a single consolidated diagram asset using Codex Vision (`prompt_graphics_union.md`), purging individual obsolete asset crops from `assets/` and replacing the constituent nodes with a single unified node.
3. Fuses adjacent narrative `prose` nodes across page boundaries, performing de-hyphenation on split words via LLM evaluation (`prompt_seam.md`).
4. Preserves structural node boundaries for tables, code listings, and headings.

## Inputs
- `workspace/03_build_raw_stream/raw_stream.json`: Master sequential node stream from Stage 03.
- `workspace/01_preprocess/page_XXXX.png`: 300 DPI raster page images for vision verification.
- `stages/04_stream_reduction/prompt_seam.md`: LLM prompt for cross-page text seams.
- `stages/04_stream_reduction/prompt_graphics_union.md`: Vision prompt for contiguous graphic union validation.

## Outputs
- `workspace/04_stream_reduction/reduced_stream.json`: Normalized, welded node stream.
- `workspace/04_stream_reduction/assets/`: Consolidated visual assets (`asset_node_XXXXX.png`, `.svg`); raw text remains in the node JSON.

## Invocation Through the Orchestrator
```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 04 --to-stage 04
```

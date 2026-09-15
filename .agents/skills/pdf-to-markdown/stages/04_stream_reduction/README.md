# Stage 04: Stream Reduction

## Objective
Performs sequential normalization across the global node stream:
1. Strips repetitive running `header` and `footer` blocks.
2. Identifies and unifies contiguous `graphic` fragments on the same page into a single consolidated diagram asset using Gemini Vision (`prompt_graphics_union.md`), purging individual obsolete asset crops from `workspace/assets/` and replacing the constituent nodes with a single unified node.
3. Fuses adjacent narrative `prose` nodes across page boundaries, performing de-hyphenation on split words via LLM evaluation (`prompt_seam.md`).
4. Preserves structural node boundaries for tables, code listings, and headings.

## Inputs
- `workspace/raw_stream.json`: Master sequential node stream.
- `workspace/01_pages/page_XXXX.png`: 300 DPI raster page images for vision verification.
- `stages/04_stream_reduction/prompt_seam.md`: LLM prompt for ambiguous cross-page text seams.
- `stages/04_stream_reduction/prompt_graphics_union.md`: Vision prompt for contiguous graphic union validation.

## Outputs
- `workspace/04_reduced_stream/reduced_stream.json`: Normalized, welded node stream.
- Updated `workspace/assets/`: Purged individual fragmented assets, replaced with consolidated diagram assets (`asset_node_XXXXX.png`, `.txt`, `.svg`).

## Standalone Invocation
```powershell
python stages/04_stream_reduction/reduce_stream.py --workspace "workspace"
```

# Stage 04: Stream Reduction

## Objective
Performs sequential normalization across the global node stream:
1. Strips repetitive running `header` and `footer` blocks.
2. Fuses adjacent narrative `prose` nodes across page boundaries, performing de-hyphenation on split words.
3. Preserves structural node boundaries for tables, graphics, code listings, and headings.

## Inputs
- `workspace/raw_stream.json`: Master sequential node stream.
- `stages/04_stream_reduction/prompt_seam.md`: LLM prompt for ambiguous cross-page text seams.

## Outputs
- `workspace/reduced_stream.json`: Normalized, welded node stream.

## Standalone Invocation
```powershell
python stages/04_stream_reduction/reduce_stream.py --workspace "workspace"
```

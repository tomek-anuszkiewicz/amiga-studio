# Stage 06: Continuation Detection

## Objective
Identifies adjacent content blocks across page boundaries (primarily tables, optionally split schematics) that belong together.
Links head nodes and continuation nodes via explicit JSON metadata (`continuation_status`, `continuation_group_id`, `continued_from`, `merged_assets`), allowing Stage 07 to synthesize unified tables without fragmentation.

## Inputs
- `workspace/05_chapter_partition/{index:02d}_{slug}.json`: Partitioned section stream files from Stage 05.
- `stages/06_detect_continuations/prompt_continuation.md`: LLM prompt for continuation verification.

## Outputs
- `workspace/06_detect_continuations/{index:02d}_{slug}.json`: Chapter streams with continuation relationships annotated on nodes.

## Standalone Invocation
```powershell
python stages/06_detect_continuations/detect_continuations.py --workspace "workspace"
```

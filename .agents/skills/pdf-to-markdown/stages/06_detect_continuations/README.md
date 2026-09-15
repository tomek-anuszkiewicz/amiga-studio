# Stage 06: Continuation Detection

## Objective
Identifies adjacent content blocks across page boundaries (primarily tables, optionally split schematics) that belong together.
Links head nodes and continuation nodes via explicit JSON metadata (`continuation_status`, `continuation_group_id`, `continued_from`, `merged_assets`), allowing Stage 07 to synthesize unified tables without fragmentation.

## Inputs
- `workspace/chapters/*.json`: Partitioned section stream files.
- `stages/06_detect_continuations/prompt_continuation.md`: LLM prompt for continuation verification.

## Outputs
- Updated `workspace/chapters/*.json` with continuation relationships annotated on nodes.

## Standalone Invocation
```powershell
python stages/06_detect_continuations/detect_continuations.py --workspace "workspace"
```

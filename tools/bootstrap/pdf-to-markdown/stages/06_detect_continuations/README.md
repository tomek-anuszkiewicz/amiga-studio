# Stage 06: Continuation Detection

Stage 02.81 Markdown/HTML table fragments bypass continuation joining. Preserve
their markup, group text, source-image references and source blocks independently;
multi-page joining of converted tables is outside this iteration.

## Objective
Identifies adjacent content blocks across page boundaries (primarily tables, optionally split schematics) that belong together.
Links head nodes and continuation nodes via explicit JSON metadata (`continuation_status`, `continuation_group_id`, `continued_from`, `merged_assets`), allowing Stage 07 to synthesize unified tables without fragmentation.

## Inputs
- `workspace/05_chapter_partition/{index:02d}_{slug}.json`: Partitioned section stream files from Stage 05.
- `stages/06_detect_continuations/prompt_continuation.md`: LLM prompt for continuation verification.

## Outputs
- `workspace/06_detect_continuations/{index:02d}_{slug}.json`: Chapter streams with continuation relationships annotated on nodes.

## Invocation Through the Orchestrator
```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 06 --to-stage 06
```

Chapter JSON includes `index`, `slug`, `title`, `target_md_file` and `nodes`.
This automatic stage transforms nodes while preserving metadata and prior-stage files.

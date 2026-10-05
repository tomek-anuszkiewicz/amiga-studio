# Stage 11: Emit Markdown

## Objective
Serializes chapter streams into standalone Markdown files formatted as `<output_dir>/{index:02d}_{slug}.md`:
1. Emits clean Markdown body content for each chapter partition.
2. Explicitly ignores and skips any segment of type `toc_heading` (eliminating redundant raw TOC banners).
3. Skips child continuation nodes whose content was synthesized into the head node.
4. Synchronizes visual assets and RAG sidecars into `<output_dir>/assets/`.

> [!NOTE]
> Publication-grade YAML frontmatter (Obsidian properties: `title`, `book`, `chapter`, `tags`) is generated downstream in Stage 12 (`12_generate_properties`).

## Inputs
- `workspace/chapters_manifest.json`: Section manifest proofread from Stage 10.
- `workspace/10_proofread_stream/{index:02d}_{slug}.json`: Proofread chapter streams.
- `workspace/assets/`: Visual crops (`.svg`, `.png`) and text sidecars (`.txt`).

## Outputs
- `<output_dir>/{index:02d}_{slug}.md`: Emitted per-section Markdown documents.
- `<output_dir>/assets/`: Synchronized visual assets and RAG sidecars.

## Invocation Through the Orchestrator
```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 11 --to-stage 11
```

# Stage 11: Emit Markdown

## Objective
Serializes chapter streams into standalone Markdown files formatted as `<output_dir>/{target_md_file}`:
1. Emits clean Markdown body content for each chapter partition.
2. Explicitly ignores and skips any segment of type `toc_heading` (eliminating redundant raw TOC banners).
3. Skips child continuation nodes whose content was synthesized into the head node.
4. Synchronizes visual assets and RAG sidecars into `<output_dir>/assets/`.

Stage 02.81 converted table fragments remain independently visible even when their
source continuation flag is true. Shared group assembly appends one collapsed
`Table text for RAG` section using saved literal `table_rag_text`, then one collapsed
`Original table image` section after the last associated source block. Captions,
footnotes and legends retain their visible source order. Copy the referenced original
crop from the carried assets bundle; do not reinfer or duplicate companion text.

> [!NOTE]
> Publication-grade YAML frontmatter (Obsidian properties: `title`, `book`, `chapter`, `tags`) is generated downstream in Stage 12 (`12_generate_properties`).

## Inputs
- `workspace/10_proofread_stream/{index:02d}_{slug}.json`: Self-contained chapter metadata and nodes from Stage 10.
- `workspace/10_proofread_stream/assets/`: Visual crops (`.svg`, `.png`) and text sidecars (`.txt`).

## Outputs
- `<output_dir>/{target_md_file}`: Emitted per-section Markdown documents.
- `<output_dir>/assets/`: Synchronized visual assets and RAG sidecars.

## Invocation Through the Orchestrator
```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 11 --to-stage 11
```

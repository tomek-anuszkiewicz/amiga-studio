# Stage 11: Emit Markdown

## Objective
Serializes chapter streams into standalone, publication-grade Markdown files formatted as `<output_dir>/{index:02d}_{slug}.md`:
1. Injects active YAML frontmatter bounded by `---` on Line 1 of each document (`title`, `section_index`, `tags`, `properties`).
2. Explicitly ignores and skips any segment of type `toc_header` (eliminating redundant raw TOC banners).
3. Skips child continuation nodes whose content was synthesized into the head node.
4. Synchronizes visual assets and RAG sidecars into `<output_dir>/assets/`.

## Inputs
- `workspace/chapters_manifest.json`: Section manifest proofread from Stage 10.
- `workspace/10_proofread_stream/{index:02d}_{slug}.json`: Proofread chapter streams.
- `workspace/assets/`: Visual crops (`.svg`, `.png`) and text sidecars (`.txt`).

## Outputs
- `<output_dir>/{index:02d}_{slug}.md`: Emitted per-section Markdown documents.
- `<output_dir>/assets/`: Synchronized visual assets and RAG sidecars.

## Standalone Invocation
```powershell
python stages/11_emit_markdown/emit_markdown.py --workspace "workspace" --output-dir "workspace/11_emit_markdown"
```

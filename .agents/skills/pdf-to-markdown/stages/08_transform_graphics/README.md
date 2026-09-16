# Stage 08: Transform Graphics

## Objective
Specialized worker for technical schematics, flowcharts, block diagrams, and illustrations:
1. Converts state machines and flowcharts into native Mermaid diagrams paired with collapsible ASCII fallback callouts (`> [!NOTE]-`).
2. Preserves physical schematics, IC pinouts, and waveforms as Obsidian wikilink embeds: `![[assets/{id}.svg]]` (or `.png`).
3. Generates comprehensive technical sidecars (`assets/{id}.png.txt`) containing signal names, timing equations, and register definitions for offline vector RAG search.

## Inputs
- `workspace/07_transform_tables/{index:02d}_{slug}.json`: Chapter streams from Stage 07.
- `workspace/assets/`: Visual crops (`.svg`, `.png`).
- `stages/08_transform_graphics/prompt_mermaid.md`: Diagram-to-Mermaid prompt.
- `stages/08_transform_graphics/prompt_rag_sidecar.md`: RAG technical sidecar prompt.

## Outputs
- `workspace/08_transform_graphics/{index:02d}_{slug}.json`: Chapter streams with rendered diagram markup in `node.rendered_markdown`.
- `workspace/08_transform_graphics/assets/{id}.png.txt`: Vector search technical sidecars.

## Standalone Invocation
```powershell
python stages/08_transform_graphics/transform_graphics.py --workspace "workspace"
```

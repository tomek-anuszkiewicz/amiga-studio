# Stage 08: Transform Graphics

## Objective
Specialized worker for technical schematics, flowcharts, block diagrams, and illustrations:
1. Converts state machines and flowcharts into native Mermaid diagrams paired with collapsible ASCII fallback callouts (`> [!NOTE]-`).
2. Preserves physical schematics, IC pinouts, and waveforms as Obsidian wikilink embeds: `![[assets/{id}.svg]]` (or `.png`).
3. Generates comprehensive technical sidecars (`assets/{id}.png.txt`) containing signal names, timing equations, and register definitions for offline vector RAG search.

## Inputs
- `workspace/chapters/*.json`: Partitioned section stream files.
- `stages/08_transform_graphics/prompt_mermaid.md`: Diagram-to-Mermaid prompt.
- `stages/08_transform_graphics/prompt_rag_sidecar.md`: RAG technical sidecar prompt.

## Outputs
- Updated `workspace/chapters/*.json` with rendered diagram markup in `node.rendered_markdown`.
- `workspace/assets/{id}.png.txt`: Vector search technical sidecars.

## Standalone Invocation
```powershell
python stages/08_transform_graphics/transform_graphics.py --workspace "workspace"
```

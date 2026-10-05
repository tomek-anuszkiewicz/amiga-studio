# Stage 08: Transform Graphics

## Objective
Specialized worker for technical schematics, flowcharts, block diagrams, and illustrations:
1. **Mermaid Precedence (Default for Dataflow & Relationships):** Converts calculation trees, effective address generation graphs, state machines, and flowcharts into native Mermaid diagrams paired with collapsible ASCII fallback callouts (`> [!NOTE]-`).
2. **Clean Box Discipline (ASCII Art for Bitfields):** Formats register bitfields (e.g. 16/32-bit registers) into compact, enclosed ASCII boxes (3–4 lines) with zero leader lines or pointer stalks; all field definitions and decoded meanings are placed strictly in structured Markdown tables/lists below.
3. **Physical Schematics & Embeds:** Preserves physical schematics, IC pinouts, and waveforms as Obsidian wikilink embeds: `![[assets/{id}.svg]]` (or `.png`).
4. **Vector RAG Sidecars:** Generates comprehensive technical sidecars (`assets/{id}.png.txt`) containing signal names, timing equations, and register definitions for offline vector RAG search.

## Inputs
- `workspace/07_transform_tables/{index:02d}_{slug}.json`: Chapter streams from Stage 07.
- `workspace/assets/`: Visual crops (`.svg`, `.png`).
- `stages/08_transform_graphics/prompt_triage.md`: Classification prompt (Mermaid vs ASCII art vs schematic).
- `stages/08_transform_graphics/prompt_mermaid.md`: Diagram-to-Mermaid prompt.
- `stages/08_transform_graphics/prompt_ascii_art.md`: Register bitfield clean box prompt.
- `stages/08_transform_graphics/prompt_rag_sidecar.md`: RAG technical sidecar prompt.

## Outputs
- `workspace/08_transform_graphics/{index:02d}_{slug}.json`: Chapter streams with rendered diagram markup in `node.rendered_markdown`.
- `workspace/08_transform_graphics/assets/{id}.png.txt`: Vector search technical sidecars.

## Standalone Invocation
```powershell
python stages/08_transform_graphics/transform_graphics.py --workspace "workspace"
```

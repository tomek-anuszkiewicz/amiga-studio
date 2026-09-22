# Graphic Classification Triage Prompt

Analyze this technical graphic.
Determine the most appropriate representation format following this strict precedence:

1. `mermaid` (DEFAULT for relationships, computation, and data flow):
   - Mandatory for all calculation graphs, effective address generation trees, arithmetic flowcharts ($+$, $\times$, pointers to memory), functional block diagrams, state transition machines, sequence timing relationships, or decision flowcharts.
   - If a diagram shows values moving, summing, scaling, indexing, or pointing to memory addresses, ALWAYS choose `mermaid`.

2. `ascii_art` (Strictly for static binary layouts):
   - Exclusively for bitfield register structures (e.g. 16/32-bit registers showing bit 15 down to 0 with bit names inside the cells), memory maps showing numerical address ranges, packet frame byte structures, or monospace character grids where rigid horizontal column alignment of bits/bytes is mandatory.
   - NEVER use `ascii_art` for arithmetic computation trees, data movement, or address generation graphs.

3. `schematic` (Preserve as original image):
   - Detailed electrical/logic circuit gate schematics, analog timing waveform graphs, complex IC pin wiring, photographs, or book cover art that cannot be cleanly modeled as a diagram.

Return a strict JSON object:
```json
{
  "type": "mermaid" | "ascii_art" | "schematic",
  "caption": "Short descriptive title"
}
```

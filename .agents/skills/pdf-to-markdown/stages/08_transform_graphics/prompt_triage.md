# Graphic Classification Triage Prompt

Analyze this technical graphic.
Determine the most appropriate representation format:
1. `mermaid`: Is it a flowchart, state transition machine, sequence diagram, or high-level functional block chart that is best expressed as executable Mermaid diagram code?
2. `ascii_art`: Is it a register bitfield layout, memory map, data structure bit/byte breakdown, packet/frame layout, or simple structural diagram that is best converted into clean, readable ASCII art?
3. `schematic`: Is it a detailed electrical/logic circuit schematic, analog timing waveform graph, complex IC pin wiring, photograph, or book cover art that must be preserved as an image?

Return a strict JSON object:
```json
{
  "type": "mermaid" | "ascii_art" | "schematic",
  "caption": "Short descriptive title"
}
```

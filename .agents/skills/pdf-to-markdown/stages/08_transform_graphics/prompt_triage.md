# Graphic Classification Triage Prompt

Analyze this technical graphic. Is it a flowchart, state diagram, or structural block chart that should be converted to Mermaid code? Or is it a detailed circuit schematic, timing waveform, IC pinout, or photographic illustration that must be preserved as an image?

Return a strict JSON object:
```json
{
  "type": "mermaid" | "schematic",
  "caption": "Short descriptive title"
}
```

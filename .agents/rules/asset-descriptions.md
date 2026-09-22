---
trigger: model_decision
description: Inspect diagrams and generate Git-tracked sidecar text descriptions (.txt) using Agent native multimodal vision.
---

## Diagram & Asset Sidecar Descriptions

All circuit diagrams, timing diagrams, pinouts, and architecture schematics in documentation assets (e.g. `Obsidian/Amiga/Reference/*/assets/`) maintain corresponding Git-tracked text description files: `<image_path>.txt`.

These sidecar files preserve diagram details without requiring cloud APIs or runtime OCR. The current `rag_qdrant` CLI indexes Markdown only and does not consume sidecar content.

### Change Detection
- An image requires a description when its `<image_path>.txt` sidecar is missing or no longer accurately reflects the image.
- The standalone indexing tool maintains Markdown hashes in the state file named by `RAG_INDEX_JSON`; Amiga project workflows do not access that state directly.

### Execution Skill
- Follow the standardized operational recipe in [`describe-diagram-assets`](../skills/describe-diagram-assets/SKILL.md) to inspect diagrams and generate `<image_path>.txt` sidecars.

### Generation Workflow (Agent Native Multimodal Vision)
When requested by the user or when assets are modified:
1. Identify images without companion sidecars using the PowerShell command in `describe-diagram-assets`.
2. Inspect the image natively using `view_file` (consuming Antigravity IDE's internal multimodal model quota, not external API keys).
3. Generate a structured, factual technical description adhering to this standard:
   ```markdown
   [Diagram: <Figure Title / Diagram Name>]
   - Subsystem / Component: <e.g. M68000 Bus Arbitration, Paula Audio, Copper Timing>
   - Circuit / Signals / Pinouts: <Extracted signal names: /AS, /DTACK, /BERR, D0-D15, etc.>
   - State & Cycle Transitions: <Timing steps, clock edges, wait states, microcode sequence>
   - Architectural Summary: <Core engineering takeaway and cycle-exact behavior>
   ```
4. Save the description into `<image_path>.txt`.
5. Keep both image and sidecar `.txt` version-controlled in Git for direct human and agent inspection.

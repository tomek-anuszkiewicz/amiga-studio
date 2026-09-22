---
name: describe-diagram-assets
description: Inspect circuit diagrams via multimodal vision and author technical sidecar text files for Amiga RAG MCP retrieval.
---

# Recipe: Technical Diagram & Asset Sidecar Generation

This skill provides the standard operational procedure for generating and synchronizing Git-tracked companion technical descriptions (`<image_path>.txt`) for circuit diagrams, timing diagrams, pinouts, and hardware schematics in [Obsidian/Amiga/Reference/](../../../Obsidian/Amiga/Reference/) per [`.agents/rules/asset-descriptions.md`](../../rules/asset-descriptions.md).

---

## 1. When to Use This Skill

Activate this skill whenever:
- Adding, replacing, or updating technical diagrams, waveforms, or pinouts in `Obsidian/Amiga/Reference/*/assets/`.
- Preparing documentation assets for Amiga RAG MCP retrieval.
- Auditing documentation diagrams to verify each has a corresponding `.txt` sidecar.

---

## 2. Tooling & Asset Registry

- **Amiga RAG Integration:** The project retrieves indexed documentation through its MCP tools.
- **Vision Inspection:** Native `view_file` tool (consuming IDE multimodal vision, 100% offline with zero external cloud API keys).

---

## 3. Step-by-Step Execution Workflow

### Step 1: Identify Diagram Assets That Need Descriptions
Audit newly added or changed images and identify any missing or inaccurate `<image_path>.txt` sidecars.

### Step 2: Inspect Image with Multimodal Vision
For each unindexed asset:
1. Open and inspect the image using `view_file`:
   ```text
   view_file(AbsolutePath="<path_to_diagram_image>")
   ```
2. Identify circuit components, active-low signals (e.g. `_AS`, `_DTACK`, `_BERR`, `_UDS`, `_LDS`), clock phases (`CCK1`, `CCK2`), state transitions, and bus cycles.

### Step 3: Author Structured Technical Sidecar (`<image_path>.txt`)
Create or update `<image_path>.txt` immediately alongside the image file, following this strict structure:
```markdown
[Diagram: <Figure Title / Schematic Name>]
- Subsystem / Component: <e.g. Paula Audio Channel DMA, 68000 Bus Arbitration, Agnus Beam Counter>
- Circuit / Signals / Pinouts: <Extracted signals: _AS, _DTACK, _VPA, _VMA, E-Clock, IPL0-IPL2, etc.>
- State & Cycle Transitions: <Timing steps, clock edges, wait-state injection, state machine sequence>
- Architectural Summary: <Core physical takeaway, cycle-exact timing rules, and hardware circuit behavior>
```

### Step 4: Preserve the Sidecar for MCP Retrieval
Save the generated sidecar beside its image. The shared cache records hashes independently; run the Amiga MCP `rag_reindex()` workflow when the changed asset is in an indexed documentation scope.

### Step 5: Verification & Version Control
1. Verify every changed diagram has an accurate `<image_path>.txt` sidecar.
2. Ensure both the diagram and its `<image_path>.txt` sidecar are tracked in Git.

---

## 4. Execution Mode: Subagent Delegation

- **Execution Host:** **Isolated Subagent** (child context sandbox).
- **Model Tier:** `Gemini Flash Low (Multimodal Vision)`
- **Context Savings:** Absorbs raster image bytes, pinout coordinate measurements, and raw OCR inspection from the main conversation.
- **Subagent Task Template:**
  - `TaskName`: "Generating Diagram Sidecars: <asset_name>"
  - `TaskSummary`: "Inspects diagram image via native multimodal vision, extracts signals/circuits, generates `<image>.txt` sidecar, and updates cache."
  - `Prompt`:
    ```markdown
    Generate technical sidecar for diagram asset: <IMAGE_PATH>.
    Follow .agents/skills/describe-diagram-assets/SKILL.md:
    1. Inspect image with `view_file`.
    2. Extract active-low signals, pinouts, timing states, and hardware behavior.
    3. Author `<image_path>.txt` technical sidecar alongside image.
    4. Preserve the sidecar beside the image, then reindex its documentation scope through the Amiga MCP server.
    5. Return strictly the Sidecar Generation Report below.
    ```
- **Return Contract (Mandatory Structured Output):**
  The subagent must conclude with this exact markdown block:
  ```markdown
  ### 📐 Diagram Asset Description Report
  - **Asset Processed:** `<image_path>`
  - **Generated Sidecar:** [`<image_path>.txt`](file:///<image_path>.txt)
  - **Extracted Signals / Pinouts:** `<comma_separated_signals>` (e.g. `_AS`, `_DTACK`, `_BERR`, `IPL0-IPL2`)
  - **Sidecar Status:** Saved beside the image and ready for MCP-backed retrieval after ingestion.
  ```

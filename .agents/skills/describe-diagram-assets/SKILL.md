---
name: describe-diagram-assets
description: Inspect circuit diagrams via multimodal vision, author technical sidecars (<image>.txt), and sync RAG cache.
---

# Recipe: Technical Diagram & Asset Sidecar Generation

This skill provides the standard operational procedure for generating and synchronizing Git-tracked companion technical descriptions (`<image_path>.txt`) for circuit diagrams, timing diagrams, pinouts, and hardware schematics in [Obsidian/Amiga/Reference/](../../../Obsidian/Amiga/Reference/) per [`.agents/rules/asset-descriptions.md`](../../rules/asset-descriptions.md).

---

## 1. When to Use This Skill

Activate this skill whenever:
- Adding, replacing, or updating technical diagrams, waveforms, or pinouts in `Obsidian/Amiga/Reference/*/assets/`.
- Preparing documentation assets for offline RAG indexing via `rag_qdrant`.
- Running an asset audit to verify all documentation diagrams have corresponding `.txt` sidecars.

---

## 2. Tooling & Asset Registry

- **Indexer CLI:** `rag_qdrant`; it owns the SHA-256 cache and detects changed sidecars during incremental indexing.
- **Vision Inspection:** Native `view_file` tool (consuming IDE multimodal vision, 100% offline with zero external cloud API keys).

---

## 3. Step-by-Step Execution Workflow

### Step 1: Detect Missing or Outdated Diagram Sidecars
List images without companion sidecars:
```powershell
Get-ChildItem "Obsidian/Amiga" -Recurse -File |
  Where-Object { $_.Extension -in '.png', '.jpg', '.jpeg', '.webp', '.svg' } |
  Where-Object { -not (Test-Path "$($_.FullName).txt") } |
  Select-Object -ExpandProperty FullName
```

When an image changed, inspect it and update its existing sidecar if the description is no longer accurate.

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

### Step 4: Incrementally Index the Updated Sidecars
The CLI records hashes while indexing; do not edit its cache directly:
```powershell
rag_qdrant "Obsidian/Amiga" --source amiga --include-dirs Design Reference
```

### Step 5: Verification & Version Control
1. Re-run the missing-sidecar command from Step 1 and inspect updated diagrams visually.
2. Verify the index and a distinctive sidecar phrase:
   ```powershell
   rag_qdrant --status
   rag_qdrant search "<distinctive sidecar phrase>" --source amiga --limit 2 --json
   ```
3. Ensure both the diagram and its `<image_path>.txt` sidecar are tracked in Git.

---

## 4. Standard Report Format

```markdown
### 📐 Diagram Asset Description Report
- **Asset Processed:** `<image_path>`
- **Generated Sidecar:** [`<image_path>.txt`](file:///<image_path>.txt)
- **Extracted Signals / Pinouts:** `<comma_separated_signals>` (e.g. `_AS`, `_DTACK`, `_BERR`, `IPL0-IPL2`)
- **Index Status:** Incremental `rag_qdrant` run completed and retrieval was verified.
```

---
trigger: model_decision
description: One-time conversion-bootstrap contract for deciding whether visual assets matter and making retained visuals searchable in Markdown and RAG.
---

# Asset Descriptions and Markdown Fallbacks

Apply this rule when bootstrapping a source-faithful HTML- or PDF-to-Markdown conversion that contains tables, images, diagrams, or ASCII art. It governs the conversion artefacts, not ordinary emulator source-code documentation.

## 1. One Bootstrap Pass

Perform one visual-asset pass after the source inventory and before bulk conversion. It is not a per-page or per-chapter task. Repeat it only when the source asset set or the conversion contract changes.

The bootstrap pass must:

1. Inspect the active Gemini workflows and record, for tables and graphics, what is automated, when vision is called, what output is written, and which task handoffs require an agent to edit or approve the result.
2. Inventory visual asset classes and decide whether each class carries information that the prose and extracted text do not already preserve.
3. Set the representation contract below before producing converted Markdown.
4. Confirm that generated sidecars and collapsible Markdown descriptions are actually present in a small representative output; configured prompts or returned-but-unwritten values are not evidence of completion.

## 2. Representation Contract

| Source visual | Required converted form |
| --- | --- |
| HTML table retained for layout or source fidelity | Keep the HTML table, then add a `<details>` block containing an equivalent GFM Markdown table. Preserve every header, cell, order, and span semantics; do not use the fallback to repair or reinterpret the source table. |
| Retained raster or vector image | Keep the image asset and create a plain-text sidecar beside it using the exact filename `<asset filename>.txt` (for example, `timing.png.txt`). Insert the same source-grounded description in a closed `<details>` block immediately after the image embed. |
| ASCII art retained as text | Keep the ASCII art in a fenced `text` block. Follow it with a closed `<details>` block describing the diagram's purpose, components, labels, and directional or positional relationships. |

Use this shape for visible descriptions:

```markdown
![[timing.png]]

<details>
<summary>Image description</summary>

The source-grounded description from `timing.png.txt`.

</details>
```

The sidecar and the collapsed text must match. If a shared generation mechanism is unavailable, compare them before delivery so that they do not drift.

## 3. When a Visual Belongs in the Output

Retain an image when it conveys non-redundant visual information: physical appearance, geometry, topology, timing shape, spatial arrangement, graph structure, or detail that cannot be faithfully represented by ordinary text, a Markdown table, Mermaid, or ASCII art. Do not retain decorative, duplicated, illegible, or semantically empty images merely because the source contains an `<img>` tag.

Prefer a native text representation for a table, flow, state machine, or bitfield when it preserves all relevant content more clearly. Preserve the original image as well when the source visual itself remains material evidence or conveys information the replacement cannot retain.

## 4. RAG-Useful Descriptions

An asset description is sufficient for RAG only when a reader who cannot see the visual can answer all applicable questions:

- What is this visual, and what question does it answer in the surrounding document?
- Which named components, labels, values, ranges, pins, signals, or axes does it contain?
- How are those elements connected, ordered, grouped, directed, or timed?
- Which stated relationship, constraint, or consequence is material to the document?

OCR labels, a generic alt text, or an aesthetic caption alone are not sufficient. Describe visible facts and source-provided context precisely; do not invent hidden behavior, uncertain values, or technical conclusions. For a visual with no searchable technical content, record the reason it was omitted rather than fabricating a sidecar.

## 5. Verification

Before bulk conversion, verify a representative table, image, and ASCII-art sample when each exists:

- The HTML table and GFM fallback contain the same data.
- Every retained image has its same-basename `.txt` sidecar and matching collapsed description.
- Each ASCII-art description explains its structure without altering the art.
- The inclusion or omission decision is source-grounded and documented in the bootstrap record.

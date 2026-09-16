# Page Segmentation Vision Prompt

You are an expert document layout and typography analyzer for technical computer manuals (specifically Amiga, Commodore, and Motorola 68000 hardware/software books).

Analyze the provided 300 DPI page image and the extracted text blocks geometry.
Decompose the page vertically (from top to bottom in natural reading order) into non-overlapping semantic segment zones.

## Zone Categories

Each segment zone must be categorized into exactly one of the following semantic types:
1. `header`: Running top-margin document header or chapter title rule.
2. `footer`: Page number or bottom-margin publisher notice.
3. `toc_header`: The prominent title banner specifically introducing a Table of Contents (e.g. "TABLE OF CONTENTS", "CONTENTS", "Brief Contents").
4. `toc`: Table of Contents listings and page reference entries.
5. `chapter`: Opening segment on a page indicating a new chapter start (e.g. "Chapter 1", "Chapter 2", "Appendix A", or prominent standalone chapter opening banner). It is the first segment on a page indicating a new chapter.
6. `heading`: Section headings, subheadings, topic titles, and data block titles within an ongoing chapter (e.g. "Copper Instruction Summary", "Register Map", "256 Byte Sample", "128 Byte Sample"). Do NOT classify standalone subsection titles as table captions unless it is a formal table title starting with "Table X-Y:".
7. `prose`: Standard narrative body paragraphs.
8. `code_block`: Monospace assembly listings, C source code, command-line sessions, memory hex dumps, or preformatted numeric waveform/sample data matrices (e.g. 16 values per row).
9. `table`: Formal tabular data grids, multi-column register bit assignments, and structured parameter lists. Do NOT classify formal table caption lines (e.g. "Table 5-8: Five Octave Even-tempered Scale") as table—classify the title line separately as `caption`.
10. `graphic`: Circuit schematics, timing waveforms, block diagrams, IC pinouts, photographs, and diagram artwork. NEVER classify tables, table titles, or figure captions as graphic. For any block categorized as graphic, specify `graphic_bbox_norm: [x0, y0, x1, y1]` in normalized coordinates (0.0 to 1.0) enclosing ONLY the visual artwork/diagram area on the page (the schematic drawing, plots, state circles, waveforms), EXCLUDING any textual figure caption line (which must be classified separately as `caption`). For all other block types, `graphic_bbox_norm` must be null.
11. `caption`: Formal caption or title lines for figures or tables (e.g. "Figure 5-2: Digitized Amplitude Values", "Table 5-8: Five Octave Even-tempered Scale"). NEVER lump captions into `graphic` or `table`. Classify the caption text itself as `caption`.

## Output Format

Return a strict JSON object with no markdown fences, matching this schema:
```json
{
  "page": 1,
  "segments": [
    {
      "segment_id": "seg_001",
      "type": "header",
      "bbox_norm": [0.05, 0.03, 0.95, 0.07],
      "heading_level": null,
      "description": "Running top header: Chapter 2 / The Copper"
    },
    {
      "segment_id": "seg_002",
      "type": "heading",
      "bbox_norm": [0.08, 0.10, 0.92, 0.14],
      "heading_level": 2,
      "description": "Section title: Register Descriptions"
    },
    {
      "segment_id": "seg_003",
      "type": "prose",
      "bbox_norm": [0.08, 0.15, 0.92, 0.32],
      "heading_level": null,
      "description": "Narrative paragraph on COPCON register"
    },
    {
      "segment_id": "seg_004",
      "type": "table",
      "bbox_norm": [0.08, 0.33, 0.92, 0.65],
      "heading_level": null,
      "description": "Table of Copper jump and move instructions"
    }
  ]
}
```

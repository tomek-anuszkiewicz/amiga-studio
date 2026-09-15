# Stage 02: Page Segmentation

## Objective
Analyzes each page vertically from top to bottom, classifying bands into explicit semantic types:
- `header`: Running top headers.
- `footer`: Page numbers, publisher bottom lines.
- `toc_header`: Banner title introducing the Table of Contents (e.g. "TABLE OF CONTENTS").
- `toc`: Table of contents listings and page references.
- `heading`: Chapter, section, and subsection headings.
- `prose`: Running narrative body paragraphs.
- `code_block`: Monospace assembly, C, or command listings.
- `table`: Tabular grids, register bitfields.
- `graphic`: Circuit schematics, flowcharts, waveforms, photos.

## Inputs
- `workspace/pages/page_XXXX.png`: 300 DPI page render.
- `workspace/pages/page_XXXX.json`: Text geometry blocks.
- `stages/02_page_segmentation/prompt.md`: Vision prompt.

## Outputs
- `workspace/segments/page_XXXX_segments.json`: Segment list with coordinates, text, and types.

## Standalone Invocation
```powershell
python stages/02_page_segmentation/segment_page.py --workspace "workspace"
```

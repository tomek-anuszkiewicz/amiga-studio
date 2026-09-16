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
- `workspace/01_preprocess/page_XXXX.png`: 300 DPI high-resolution page render.
- `workspace/01_preprocess/page_XXXX.json`: Text geometry blocks and normalized coordinates.
- `stages/02_page_segmentation/prompt.md`: Vision segmentation prompt.
- `stages/02_page_segmentation/prompt_empty_page.md`: Vision prompt for empty/unsegmented pages.

## Outputs
- `workspace/02_page_segmentation/page_XXXX_segments.json`: Segment list with coordinates, text, and classified types.

## Standalone Invocation
```powershell
python stages/02_page_segmentation/segment_page.py --workspace "workspace"
```

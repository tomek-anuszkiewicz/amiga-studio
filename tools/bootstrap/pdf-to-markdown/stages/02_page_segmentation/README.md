# Stage 02: Page Segmentation

Before deleting outputs or making model requests, this worker validates the
prepared-PDF identity and the exact Stage 01 PNG/JSON page set, hashes, text,
geometry and raster transforms. The existing model request is unchanged;
the full text-JSON request redesign belongs to roadmap 1.1.

## Objective
Analyzes each page vertically from top to bottom, classifying bands into explicit semantic types:
- `header`: Running top headers.
- `footer`: Page numbers, publisher bottom lines.
- `toc_header`: Banner title introducing the Table of Contents (e.g. "TABLE OF CONTENTS").
- `toc`: Table of contents listings and page references.
- `thumb_index`: Printed edge tabs, chapter bookmark tabs, and thumb navigation markers along page margins.
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

## Invocation Through the Orchestrator
```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 02 --to-stage 02
```

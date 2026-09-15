# Stage 01: Preprocess

## Objective
Deconstructs a physical input PDF into atomic, per-page representations to serve as clean ground truth for all downstream stages.

## Produced Artifacts
- `workspace/pages/page_XXXX.pdf`: Single-page vector PDF for high-precision vector clipping.
- `workspace/pages/page_XXXX.png`: 300 DPI high-resolution raster image for Vision LLM classification.
- `workspace/pages/page_XXXX.json`: Text geometry extraction (`fitz.get_text("blocks")`) with word spans and bounding boxes.
- `workspace/manifest.json`: Document index, dimensions, and page list.

## Standalone Invocation
```powershell
python stages/01_preprocess/preprocess.py --pdf "path/to/manual.pdf" --workspace "workspace"
```

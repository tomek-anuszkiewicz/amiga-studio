# Stage 01: Preprocess

## Objective
Deconstructs a physical input PDF into atomic, per-page representations to serve as clean ground truth for all downstream stages.

## Inputs
- `<manual.pdf>`: Source PDF document passed via `--pdf`.

## Outputs
- `workspace/01_preprocess/page_XXXX.pdf`: Single-page vector PDF for high-precision vector clipping.
- `workspace/01_preprocess/page_XXXX.png`: 300 DPI high-resolution raster image for Vision LLM classification.
- `workspace/01_preprocess/page_XXXX.json`: Text geometry extraction (`fitz.get_text("blocks")`) with word spans and bounding boxes.
- `workspace/pages_manifest.json`: Document index, dimensions, and page list.

## Standalone Invocation
```powershell
python stages/01_preprocess/preprocess.py --pdf "path/to/manual.pdf" --workspace "workspace"
```

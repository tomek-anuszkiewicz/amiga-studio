# Stage 03: Build Raw Stream

## Objective
Aggregates segmented page node files into a global, flat, sequential stream `raw_stream.json`.
Simultaneously extracts initial visual clips (SVG vector or 300 DPI PNG with a 10% safety margin) and raw PDF text dumps for all `table` and `graphic` nodes into `workspace/assets/`.

## Inputs
- `workspace/segments/page_XXXX_segments.json`: Per-page segment files.
- `workspace/pages/page_XXXX.pdf`: Vector PDF for vector asset clipping.
- `workspace/pages/page_XXXX.png`: Raster renders for fallback cropping.

## Outputs
- `workspace/raw_stream.json`: Master sequential node stream.
- `workspace/assets/asset_node_XXXXX.svg` / `.png`: Cropped visual assets.
- `workspace/assets/asset_node_XXXXX.txt`: Underlying text characters extracted from within the bounding box.

## Standalone Invocation
```powershell
python stages/03_build_raw_stream/build_stream.py --workspace "workspace"
```

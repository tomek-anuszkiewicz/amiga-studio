# Stage 03: Build Raw Stream

## Objective
Aggregates segmented page node files into a global, flat, sequential stream `raw_stream.json`.
Simultaneously extracts initial visual clips (SVG vector or 300 DPI PNG with a 10% safety margin) and raw PDF text dumps for all `table` and `graphic` nodes into `workspace/03_build_raw_stream/assets/`.

## Inputs
- `workspace/02_page_segmentation/page_XXXX_segments.json`: Per-page segment files from Stage 02.
- `workspace/01_preprocess/page_XXXX.pdf`: Vector PDF for vector asset clipping.
- `workspace/01_preprocess/page_XXXX.png`: Raster renders for fallback cropping.

## Outputs
- `workspace/03_build_raw_stream/raw_stream.json`: Master sequential node stream.
- `workspace/03_build_raw_stream/assets/asset_node_XXXXX.svg` / `.png`: Cropped visual assets.
- `workspace/03_build_raw_stream/assets/asset_node_XXXXX.txt`: Underlying text characters extracted within bounding boxes.

## Standalone Invocation
```powershell
python stages/03_build_raw_stream/build_stream.py --workspace "workspace"
```

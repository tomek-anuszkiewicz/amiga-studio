# Stage 03: Build Raw Stream

## Objective
Aggregates segmented page node files into a global, flat, sequential stream `raw_stream.json`.
Simultaneously extracts initial visual clips (SVG vector or 300 DPI PNG with a 10% safety margin) for all `table` and `graphic` nodes into `workspace/03_build_raw_stream/assets/`. Text remains in each JSON node's `raw_text` field; separate raw-text files are not created.

## Inputs
- `workspace/02_page_segmentation/page_XXXX_segments.json`: Per-page segment files from Stage 02.
- `workspace/01_preprocess/page_XXXX.pdf`: Vector PDF for vector asset clipping.
- `workspace/01_preprocess/page_XXXX.png`: Raster renders for fallback cropping.

## Outputs
- `workspace/03_build_raw_stream/raw_stream.json`: Master sequential node stream, including `raw_text` from segmentation.
- `workspace/03_build_raw_stream/assets/asset_node_XXXXX.svg` / `.png`: Cropped visual assets.

## Invocation Through the Orchestrator
```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 03 --to-stage 03
```

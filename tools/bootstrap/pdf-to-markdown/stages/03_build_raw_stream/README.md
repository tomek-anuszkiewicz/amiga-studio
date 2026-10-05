# Stage 03: Build Raw Stream

## Objective
Aggregates segmented page node files into a global, flat, sequential stream `raw_stream.json`.
Simultaneously crops PNG assets for all `table`, `graphic`, and `code_block` nodes into `workspace/03_build_raw_stream/assets/`. Text remains in each JSON node's `raw_text` field; separate raw-text files are not created.

## Inputs
- `workspace/02_page_segmentation/page_XXXX_segments.json`: Per-page segment files from Stage 02.
- `workspace/01_preprocess/page_XXXX.png`: Required source raster for cropping.
- `workspace/01_preprocess/page_XXXX.json`: Required page dimensions for scaling point coordinates to actual PNG pixels.

## Outputs
- `workspace/03_build_raw_stream/raw_stream.json`: Master sequential node stream, including `raw_text` from segmentation.
- `workspace/03_build_raw_stream/assets/asset_node_XXXXX.png`: Cropped visual assets at the source PNG resolution.

The shared cropper also serves Stage 04 when fragments are merged. It converts padded point bounds using the actual PNG width and height, rounds outward to integer pixels, and clamps to page boundaries and the existing vertical neighbor limits. Missing PNG/JSON inputs or invalid/empty crops fail the stage; there is no PDF fallback or repeated rendering. New visual crops are PNGs; SVG is not extracted from the source PDF.

## Invocation Through the Orchestrator
```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 03 --to-stage 03
```

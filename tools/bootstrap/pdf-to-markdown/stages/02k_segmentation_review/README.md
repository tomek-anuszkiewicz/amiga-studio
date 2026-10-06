# Stage 02k: Graphical Segmentation Review

Render a deterministic review PNG after Stage 02. Keep the original page PNG
resolution and add a white panel of equal width on the right, producing a
`2W x H` image. Each segment gets a 2-pixel frame containing its bounding box,
rounded outward through Stage 01's recorded PDF-point-to-pixel transform.
Frames stop at the page edge.

Numbered labels follow the segment array's JSON order. Each shows the segment
type and, when present, `heading_level`. Matching colors and connector lines
link labels to frames. Labels are never sorted by bounding-box position: the
array is the intended downstream reading stream. Leaders start at frame centers,
reach a common vertical boundary and then run straight to the ordered labels.
Crossings in that panel expose inversions between stream order and vertical box
order for the user to assess. Coincident boxes share a port; their multiple labels
remain visible. The renderer does not hide inversions by moving labels.
Coordinates and source prose are not printed. Empty
segment lists produce the page with an empty panel. This is a user review
artifact; it does not modify segmentation or run inference.

The orchestrator validates completed predecessor identities and hashes. The
worker validates Stage 01 PNG/JSON pairs and the exact Stage 02 page set before
rendering. Output remains in `workspace/02k_segmentation_review/` as
`page_XXXX_review.png`, with normal completion, status and restart tracking.

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --workspace "<WORKSPACE>" --config "tools/bootstrap/pdf-to-markdown/config.yaml" --from-stage 02k --to-stage 02k
```

Stage 02k is also available through `--run-deterministic` when it is the next
ready stage. Later conversion stages still consume the original Stage 02 JSON.

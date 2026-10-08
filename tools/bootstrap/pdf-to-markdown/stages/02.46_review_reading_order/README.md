# Stage 02.46: Review suspicious page reading order

Read only completed resolved predecessors through 02.45 and the original Stage 01
PNG. Filter physical pages before detection. In current array order, flag every
pair whose earlier box is entirely to the right of the later box and whose
vertical intervals overlap (positive overlap, without pixel tolerances).
This bounded heuristic can flag legitimate columns or tall boxes and misses
vertically disjoint inversions and other layout problems. It never sorts geometry.

For each suspicious selected page, make one isolated read-only Codex request with
the complete frozen page JSON, every current ordinal/ID/bbox, all flagged pairs,
the complete original PNG and a fresh labeled copy at original detail. Diagnostic
labels include every object individually, including page furniture; they never
reuse grouped or hidden Stage 02.5 labels. The original remains available for text.
Pages without suspicious pairs make no requests. Source content is conversion
data, never tool instructions. The [prompt](prompt.md) lets the model retain order.

The response is `{"ordered_segment_ids": ["<existing ID>", "..."]}`.
Require a complete permutation: missing, duplicate or unknown IDs fail normally
without guessed repairs. Move original objects; preserve IDs and every object
field, including text, type, bbox, continuation, heading level, rotation and
lineage. Preserve page metadata. Changed array order alone produces a sparse
complete-page `page_NNNN_segments.json`; retained order produces no override.

`diagnostics/page_NNNN_input.png` retains the annotated request image;
`diagnostics/page_NNNN_order.json` records suspicious pairs and before/after IDs.
Diagnostics stay separate from source pixels and published content. A requesting
diagnostic is not successful stage execution; consult `stage_status.json`.
Worker output and pipeline metrics record live requests and cache hits. Cache
identity includes exact ordered JSON, pairs, both images, prompt/schema and model
settings through the existing shared client.

Register after 02.45 and before 02.5. Review 02.5/filtering 02.8 resolve this layer;
subsequent table groups, export and stream workers preserve array order. Absent
optional status skips the layer; failed/running status blocks consumers. Restart
clears this stage and all later artifacts, retaining predecessors and compatible
cache. Attempt-local config takes precedence; initialize only missing stage
model/effort values with `gpt-6.1-sol` / `medium`.

Run through [the pipeline](../../README.md), using a named attempt:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<BOOK>/workspace/<ATTEMPT>" --config "<CONFIG>" --page-ranges "82" --from-stage 02.46 --to-stage 02.5
```

Successful execution does not certify reading order. The user assesses the
selected fragment; further pages and publication require a separate request.

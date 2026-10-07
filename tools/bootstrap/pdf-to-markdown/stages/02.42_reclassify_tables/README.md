# Stage 02.42: Reclassify tables

Review every resolved `table` using its JSON/bbox, frozen page context and the
complete unmodified Stage 01 PNG at original detail. Apply `--page-ranges` before
collecting context or requesting inference. The shared Codex transport, cache,
ChatGPT authentication and attempt-local model/effort settings are reused.

The [prompt](prompt.md) applies the same rules to every candidate: keep genuine
tables, represent unsuitable drawn compositions as `graphic`, transcribe prose
and monospaced code, or split independent elements into ordered existing types.
Tables with different cell background colors become `graphic` even when HTML
could represent them. White and shaded cells count as different backgrounds.

`retain` preserves the source object exactly. `replace` returns a nonempty list
of complete segment objects. A singleton preserves the original bbox and
continuation flag. Split children use model-provided tight boxes and continuation
flags in source reading order. The first child retains the source ID; later IDs
use collision-free `_part_N` suffixes. Replacements record `source_segment_ids`
and `replacement_stage`; all surrounding objects remain unchanged.

Prose/code and other textual children receive complete `md_text` during this
stage. Tables and graphics retain deferred content processing. Graphic outcomes
carry `image_only: true`: Stage 03 copies the flag into metadata and crops exact
original pixels, Stage 04 preserves them as independent objects, and Stage 08
keeps their raster rather than reconverting them into Mermaid or ASCII. Existing
graphic sidecar processing remains available. Stage 02.9 also exports their
original-PNG crop through the existing graphic path.

Only changed pages receive complete-page overrides. Resolution is
`02 -> 02.4 -> 02.41 -> 02.42 -> 02.43 -> 02.44`; this worker reads only predecessors. Absent
optional status skips that layer, completed status enables overrides, and
running/failed status blocks direct consumers. Review 02.5 and filtering 02.8
use the shared resolver; 02.81 sees only remaining tables after 02.8.

Restarting 02.42 clears this stage and every later stage, preserving predecessors
and compatible cache. Use the pipeline for execution/status/cleanup:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --page-ranges "39,40,62,63,94,144" --from-stage 02.42 --to-stage 02.5
```

No local geometry/schema or conversion-quality gate is introduced. Successful
execution is separate from the user's assessment of classification and fidelity.

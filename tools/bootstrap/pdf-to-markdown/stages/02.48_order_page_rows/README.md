# Stage 02.48: Deterministic ordering within page rows

Read physically selected page JSON through completed sparse predecessors up to
02.47. This worker needs no PNG, model, prompt or inference configuration.

Freeze the predecessor `segments` array. Sort temporary references by top
(`bbox[1]` in original Stage 01 PNG pixels), retaining predecessor order for ties.
Start each group at the smallest unassigned top and keep that anchor fixed.
Include subsequent tops while `top - anchor <= ROW_TOP_TOLERANCE_PX`, initially
7 pixels. Tops 100 and 107 share a group; 108 and 114 start the next group.
This is a maximum 7-pixel separation, with no midpoint band or neighbor chaining.

For every group with at least two objects, sort its original objects by ascending
X (`bbox[0]`, left to right), with equal X retaining predecessor order. Put them into the group's occupied
array positions in ascending position order. Apply the rule to every object type
and any group size. Keep singletons, unrelated slots, IDs, all object fields and
page metadata unchanged. Nonadjacent members remain in their original slots;
the page is not globally sorted and objects are not gathered into contiguous runs.
The operation is deterministic and idempotent.

Save a complete `02.48_order_page_rows/page_NNNN_segments.json` using shared
artifact IO only when the array order changes. Console output reports the number
of multi-object groups and changed pages. No diagnostics or request images are
retained. Shared artifact IO retains its existing compact-object semantics.

Review 02.50 and filtering 02.80 resolve this sparse layer after 02.47; downstream
consumers preserve the resolved sequence. Missing optional status skips it;
failed/running status blocks consumers. Restart through the orchestrator clears
02.48 and all later artifacts while preserving predecessors and cache. This also
removes obsolete overrides when a rerun finds no order change.

At the user's request, run a selected fragment in an existing named attempt:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<BOOK>/workspace/<ATTEMPT>" --config "<CONFIG>" --page-ranges "<PAGES>" --from-stage 02.48 --to-stage 02.48
```

Use `--to-stage 02.50` when review regeneration is also requested. Technical
ordering checks do not certify transcription quality or resolve every layout
observation. The Y tolerance determines row membership; X determines the order
written to the JSON array within each row's occupied positions.

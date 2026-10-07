# Stage 02.41: split grouped table objects

After completed Stage 01/02, read page objects with completed Stage 02.4
overrides. For each existing `table` on a selected physical page, send one
vision request with the complete unmodified original Stage 01 PNG, dimensions,
the target's complete JSON and frozen page objects as read-only context.
Apply `--page-ranges` before opening pages or issuing requests.

The [prompt](prompt.md) first asks whether the target visually represents a
genuine table, rather than accepting its input type as proof. Aligned register
descriptions, character-built layouts and embedded lookup snippets do not by
themselves justify splitting the composition. A non-table or uncertain target
is returned unchanged without reclassification. It then asks whether the target
combines independent tables,
including side-by-side layouts. Columns, internal sections and nested subtables
do not automatically require splitting. The response is `{"tables": [...]}`:
one item retains the original object unchanged; multiple items replace only
that target with individual table objects in model-provided reading order.
Empty responses fail rather than removing the target. Table transcription
remains deferred to Stage 02.81.

Each split carries its own model-provided original-PNG pixel `bbox`,
model-provided `continuation` and empty `md_text`. For each table split out,
the prompt asks the model to set true when it judges the table to continue
another, including a neighboring table on the same page. Independent tables
receive false. The worker preserves the model's per-table decision.
Singleton responses still preserve the predecessor object unchanged, including
its existing continuation flag. IDs retain the original for the first child, then receive
deterministic collision-free `_table_N` suffixes. Source-segment lineage and
replacement-stage metadata accompany each child. Other objects remain unchanged.
There is no generic local schema/geometry gate or quality certification.

Write complete `page_NNNN_segments.json` overrides only for changed pages.
No tables or only singleton responses means successful empty output. Missing
imagery, transport and parsing failures fail normally. Counts go to the console;
no request images, response dumps or diagnostic reports are created. Requests
reuse ChatGPT authentication, original image detail, cache, metrics and timeout.

The shared resolver applies `02 -> 02.4 -> 02.41 -> 02.42 -> 02.43 -> 02.44` before review 02.5 and
filtering 02.8. Absent optional status means skipped; running/failed status
blocks consumers. Stage 02.41 reads only predecessor layers, never itself.
Filtering remains authoritative for Stage 02.81 and later exports.

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --page-ranges "50,54" --from-stage 02.41 --to-stage 02.41
```

Restart clears 02.41 and every later stage, preserving 01/02/02.4 and the
request cache. Restarting 02.4 also clears 02.41. Existing attempt model/effort
settings stay intact; missing settings initialize to `gpt-6.1-sol` / `medium`
only when this stage is selected. The user selects fragments and assesses splits.

The prompt/schema change retires responses cached under the former false-only rule.
Existing attempt artifacts require an explicit restart at 02.41 to regenerate;
that restart also clears 02.5 and all later outputs.

# Stage 02.43: Reclassify code blocks

Review each resolved `code_block` using its JSON/bbox, frozen page objects and
the complete unmodified Stage 01 PNG at original detail. Pages without code
blocks make no model requests. Existing attempt model/effort settings are
preserved; missing stage settings default to `gpt-6.1-sol` / `medium`.

The model returns `retain`, `prose` or `table`. Retention preserves the source
exactly; prose includes a faithful Markdown transcription; tables receive empty
`md_text` and are transcribed later by 02.81. A replacement preserves the source
ID, bbox, continuation flag and order, with source lineage and replacement stage.
Only changed pages receive complete-page overrides.

Parameter descriptions pairing names with values or meanings, including
INPUT/OUTPUT register descriptions, use prose even when aligned in columns.
Preserve labels, values and qualifications as paragraphs or meaningful lists;
tables remain appropriate for matrices with additional row/column relationships.

The shared resolver applies `02 -> 02.4 -> 02.41 -> 02.42 -> 02.43` for review
02.5 and filtering 02.8. This worker reads only predecessor layers. Absent
optional status skips a layer; failed/running status blocks consumers. Physical
page selection applies before candidate collection or inference. Restarting
02.43 clears it and all later stages, preserving predecessors and request cache.

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --page-ranges "76,78,81,82,89,91,95,145,146,151,158,159" --from-stage 02.43 --to-stage 02.5
```

Inspect changed JSON under `02.43_reclassify_code_blocks/` and the annotated
images and ordered labels under `02.5_page_conversion_review/`. A successful
run establishes execution only; the user evaluates conversion quality.

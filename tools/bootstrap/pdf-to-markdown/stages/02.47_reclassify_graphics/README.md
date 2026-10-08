# Stage 02.47: Reclassify graphics

Reassess every resolved `graphic` using its JSON/bbox, frozen page objects and
the complete unmodified Stage 01 PNG at original detail. Apply page selection
before requests. The initial [prompt](prompt.md) chooses `retain`, `table`,
`code_block` or `prose` for the whole target, without splitting. Tables with
mixed cell background colors remain graphics, matching Stage 02.42.

Retention preserves the object exactly. A replacement preserves its ID, bbox,
continuation and order, removes graphic-only rotation metadata, and transcribes
prose/code immediately. Tables retain empty `md_text` for Stage 02.81.
New code blocks are transcribed here because Stage 02.45 has already run.
Only changed pages receive full-page overrides.

Resolution appends 02.47 after 02.46 before review/filtering and downstream
consumers. The worker reads only predecessors. Absent optional status skips the
layer; failed/running status blocks consumers. Restart clears 02.47 and later
stages, preserving predecessors and the shared request cache. Existing attempt
model/effort settings remain; missing settings initialize to gpt-6.1-sol/medium.
Pages without graphics make no requests. Use the pipeline:

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --page-ranges "35" --from-stage 02.47 --to-stage 02.50
```

The user selects fragments and assesses classification quality. Successful
execution does not certify fidelity; the initial prompt remains subject to review.

# Stage 10: Proofread Chapter Metadata and Nodes

Read self-contained `09_transform_prose/{index:02d}_{slug}.json` chapters, correct
titles with Codex and harmonize them with primary heading nodes. Derive canonical
slugs and target Markdown filenames, preserving collision handling and distinct
index-zero preface/TOC identities.

Write `10_proofread_stream/{index:02d}_{slug}.json` with `index`, `slug`, `title`,
`target_md_file` and `nodes`. Stage 09 files remain unchanged. Stage 11 reads these
outputs directly, without a chapter registry or older-stage fallback.

```powershell
python tools/bootstrap/pdf-to-markdown/pipeline.py --pdf "<PDF>" --workspace "<WORKSPACE>" --config "<CONFIG>" --from-stage 10 --to-stage 10
```

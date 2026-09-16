# Stage 10: Proofread Chapter Streams & Manifest

## Objective
Proofreads chapter streams and manifest metadata prior to Markdown serialization:
1. Proofreads and corrects chapter/section titles in `workspace/chapters_manifest.json` using Gemini LLM (fixing OCR typos, accidental split words, and irregular spacing).
2. Harmonizes chapter titles with the primary heading nodes in the chapter streams.
3. Derives clean, canonical slugs and target Markdown filenames directly in `chapters_manifest.json`.
4. Performs an editorial proofreading pass on stream node texts to eliminate OCR character substitutions and spacing anomalies.
5. Emits normalized chapter streams to `workspace/10_proofread_stream/{index:02d}_{slug}.json` and updates `workspace/chapters_manifest.json`.

## Inputs
- `workspace/chapters_manifest.json`
- `workspace/09_transform_prose/*.json` (or latest formatted chapter streams)

## Outputs
- `workspace/10_proofread_stream/{index:02d}_{slug}.json`: Proofread chapter streams.
- `workspace/chapters_manifest.json`: Updated manifest with proofread titles, clean slugs, and target filenames.

## Standalone Invocation
```powershell
python stages/10_proofread_stream/proofread_stream.py --workspace "workspace"
```

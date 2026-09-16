# Stage 13: Markdown OCR Proofreading & Error Correction

## Purpose
Performs a global, section-by-section proofreading pass on canonical Markdown files (`workspace/12_canonical_markdown/*.md`) using Gemini LLM. Fixes misread OCR characters, split numbers, and punctuation glitches without altering Markdown document structure. Finalizes the publication vault into `<output_dir>`.

## Input
- `workspace/12_canonical_markdown/*.md`
- `workspace/12_canonical_markdown/assets/*`

## Output
- `workspace/13_proofread_markdown/*.md`
- `workspace/13_proofread_markdown/assets/*`


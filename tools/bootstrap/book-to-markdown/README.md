# Book-to-Markdown

This directory starts the planned deterministic chapter generator. It will consume prepared data from [PDF-to-Markdown](../pdf-to-markdown/README.md) and authored JSON recipes. The generator is not implemented yet.

## Chapter proofreading

The initial [proofreading instruction](prompts/proofreading.md) is intended for manual agent processing of an already generated Markdown chapter. Supply that instruction, the chapter and relevant source pages or images when transcription needs verification. Keep the generated source chapter available for comparison and retain the edited copy outside generator-managed output cleanup.

The instruction combines source-faithful OCR correction with limited Markdown formatting. It preserves table structure, code/ASCII alignment, chapter organization, links and recipe-owned source references. Image/table descriptions and optional highlighting remain separately requested tasks.

This draft adapts the [Stage 09 formatting prompt](../pdf-to-markdown/stages/09_transform_prose/prompt.md), [formatting worker](../pdf-to-markdown/stages/09_transform_prose/format_prose.py), [Stage 10 proofreading prompt](../pdf-to-markdown/stages/10_proofread_stream/prompt.md) and [proofreading worker](../pdf-to-markdown/stages/10_proofread_stream/proofread_stream.py). It does not retain their per-node JSON workflow, legacy TOC delimiters, inferred section naming or automatic slug/filename changes. Further scope reduction follows user review.

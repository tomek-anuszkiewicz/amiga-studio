# Book-to-Markdown

This directory starts the planned deterministic chapter generator. It will consume prepared data from [PDF-to-Markdown](../pdf-to-markdown/README.md) and authored JSON recipes. The generator is not implemented yet.

## Chapter proofreading

The initial [proofreading instruction](prompts/proofreading.md) is intended for manual agent processing of an already generated Markdown chapter. Supply the instruction and chapter; the agent can inspect linked images when needed. Keep the generated source chapter available for comparison and retain the edited copy outside generator-managed output cleanup.

The instruction combines source-faithful OCR correction with limited Markdown formatting, including inline-code emphasis of technical terms. It preserves table structure, code/ASCII alignment, chapter organization and links. HTML blocks use HTML formatting and escaping rather than Markdown inline syntax. Image/table descriptions remain separately requested tasks.

This draft preserves chapter Markdown structure and filenames during explicitly
requested proofreading and formatting. Its prompts are self-contained.
Further scope reduction follows user review.

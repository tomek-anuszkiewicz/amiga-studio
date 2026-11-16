# HTML-to-Markdown Bootstrap Converter

This Python command-line program converts HTML documents (Microsoft Word exports, vintage web pages, and downloaded multi-page manuals) into Markdown for Obsidian and GitHub documentation. It prepares the initial reference knowledge base; subsequent project work uses the generated Markdown and assets.

`pipeline.py` orchestrates file processing and calls Codex through the shared [conversion client](../conversion/client.py). LLM transcription results are not guaranteed to be deterministic. Conversion instructions live in [`references/llm-transcription-prompt.md`](references/llm-transcription-prompt.md).

HTML-owned [support](support/__init__.py) implements strict YAML loading,
configuration/model checks, validated text requests and protected publication.
The shared conversion layer owns authentication, request execution, completion
handling, response caching and metrics.

Agent development guidance is in the [bootstrap instructions](../AGENTS.md#converter-development).

Model requests contain the transcription prompt and source HTML/title. All
conversion instructions live in the prompt file; the shared transport clears
base/developer instructions and sends no output schema or additional examples.

## Quick Start

Install the pinned SDK/runtime and dependencies, then sign in to Codex with ChatGPT. API-key mode is rejected:

```powershell
python -m pip install -r tools/bootstrap/html-to-markdown/requirements.txt
codex login
```

Run from the repository root:

```powershell
python tools/bootstrap/html-to-markdown/pipeline.py `
  --input "<HTML_FILE_OR_CRAWL_DIRECTORY>" `
  --document-name "<DOCUMENT_TITLE>"
```

The input is a local HTML file or an already downloaded crawl directory. Markdown
and referenced images stay in `<source-directory>/workspace/html_to_markdown/`.
Relative links to local `.s` assembly sources are expanded into named code listings
before transcription, for both single articles and crawls. Keep these files beside
the referring HTML (or in its subdirectories); missing files and paths outside that
source directory stop conversion. The prompt requires complete `m68k` listings
inside the referring Markdown section, so no separate `.s` output link is needed.
For a bootstrap crawl under `<book>-tmp/live/`, this workspace belongs to `<book>-tmp/`.
The pipeline no longer accepts `--output-dir` or mirrors Markdown into `Reference`.
Add `--publish` to copy finished Markdown and assets into the sibling book directory
without `-tmp`. Publishing requires an absent or empty destination; occupied
directories and links are rejected before inference and checked again before copying.
Metrics and downloaded sources remain in staging. The adjacent `config.yaml` selects
`llm.stages.html_to_markdown.model` and `reasoning_effort`, with separate timeout and
concurrency settings. Use `--config` for another explicit YAML file and `--cache-dir`
for a disposable pilot cache. The baseline is `gpt-6.1-sol` / `medium`, concurrency 1.
Invalid configuration, authentication and inference failures stop conversion; no PDF
configuration or DOM result substitutes for Codex.

The shared client caches only completed, validated responses under `tools/bootstrap/.cache/codex` and writes `.conversion-metrics.json` beside the output. Cached usage belongs to the original request. See the [conversion client README](../conversion/README.md) for execution and cache behavior.

Optional page rendering requires Chrome, Chromium, or Edge and PyMuPDF. Image crop extraction requires Pillow. See the operational workflow below for these tools and link validation.

> [!IMPORTANT]
> **Conversion Model:**
> HTML documents vary in layout tables, non-standard CSS, and nested tags. This converter uses LLM transcription for document text and structure, with Python scripts handling assets and output processing.
> 
> **The LLM-Driven Pipeline:**
> 1. Download/extract all referenced images to `assets/` and create technical sidecars.
> 2. Pass the HTML source (or rendered page PNGs) directly to the LLM with the unified instruction prompt in `references/llm-transcription-prompt.md`.
> 3. Resolve image placeholders and audit links.

---

## Conversion contract

The [transcription prompt](references/llm-transcription-prompt.md) is the sole
owner of content fidelity, metadata, formatting hierarchy, crawl consolidation
and table-of-contents rules. Review or change those inference requirements there;
this README owns invocation, preparation and publication.

## 7. Operational Workflow

Use `pipeline.py` for normal conversion. The following helper commands support individual preparation, inspection, and validation steps; they are not prerequisites for manually coordinating every pipeline run.

### Phase 1: Download & Prepare Assets
Run `download_assets.py` to extract, copy, or download all images referenced in the HTML document (or all HTML files in a crawl directory) to `assets/` and generate technical `.txt` sidecars:
```powershell
# For single HTML file:
python tools/bootstrap/html-to-markdown/scripts/download_assets.py `
  --html "Obsidian/Amiga/Reference/DocFolder/doc.html" `
  --assets-dir "Obsidian/Amiga/Reference/html-sandbox/assets"

# For multi-page HTML directory crawl:
Get-ChildItem "Obsidian/Amiga/Reference/DocFolder/*.html" | ForEach-Object {
  python tools/bootstrap/html-to-markdown/scripts/download_assets.py `
    --html $_.FullName `
    --assets-dir "Obsidian/Amiga/Reference/html-sandbox/assets"
}
```
*(Optional)* If visual page layout inspection is needed, render high-res page PNGs:
```powershell
python tools/bootstrap/html-to-markdown/scripts/html_to_pages.py `
  --input "Obsidian/Amiga/Reference/DocFolder/doc.html" `
  --output-dir "Obsidian/Amiga/Reference/html-sandbox/pages" `
  --resolution 200
```

### Phase 2: LLM Transcription
The pipeline sends the [transcription prompt](references/llm-transcription-prompt.md)
and the selected HTML/title or rendered pages. The prompt defines the resulting
Markdown, including metadata, code, tables, callouts, contents and image placeholders.

### Phase 3: Resolve Image Placeholders
Run `replace_placeholders.py` to verify assets, update image links, and guarantee sidecars:
```powershell
python tools/bootstrap/html-to-markdown/scripts/replace_placeholders.py `
  --markdown "Obsidian/Amiga/Reference/html-sandbox/doc.md" `
  --assets-dir "Obsidian/Amiga/Reference/html-sandbox/assets" `
  --source-assets-dir "Obsidian/Amiga/Reference/DocFolder" `
  --pages-dir "Obsidian/Amiga/Reference/html-sandbox/pages"
```

### Phase 4: Link Integrity & Asset Audit
Run `validate_links.py` to verify 100% link, anchor, and image resolution:
```powershell
python tools/bootstrap/html-to-markdown/scripts/validate_links.py `
  "Obsidian/Amiga/Reference/html-sandbox/doc.md"
```
Target: **100% PASS (0 broken files, 0 broken anchors, 0 warnings)**.

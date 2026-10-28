# Book-to-Markdown

This program imports completed [PDF-to-Markdown](../pdf-to-markdown/README.md)
preparation into a local book editor and exports chapter Markdown. The editor
shows the source PDF alongside Accepted and Working versions. Import and review
do not acquire a PDF or run the preparation stages.

Run the commands below from the repository root.

## Local HTML review

### Background server on Windows

For an already imported book, use the controller so the console remains free:

```powershell
./tools/bootstrap/book-to-markdown/review-server.ps1 start
./tools/bootstrap/book-to-markdown/review-server.ps1 status
./tools/bootstrap/book-to-markdown/review-server.ps1 restart
./tools/bootstrap/book-to-markdown/review-server.ps1 stop
```

The controller defaults to
`tools/bootstrap/books/test-book-example-4567/78e52f075e17` on port 8765.
Use `-Book <path>` for another variant and `-Port <number>` for another port.
Open `http://127.0.0.1:8765/`; the status endpoint is
`http://127.0.0.1:8765/api/status`. Process information and logs are in
`.agent/tmp/book-review/<port>/`. `status` distinguishes an HTTP response
from pending rendering and render errors.

Restart after changing review Python code or `review/document.css`; then check
`/` and `/api/status`. Reload the browser after changing viewer HTML, CSS or
JavaScript. Working book data is picked up by the render queue. Stop a foreground
`serve` process with Ctrl+C before using the controller on the same port. The
controller is a background process, not an automatically started Windows service.

### Starting a new review

Install the local review dependencies once:

```powershell
python -m pip install -r tools/bootstrap/book-to-markdown/requirements.txt
npm install --prefix tools/bootstrap/book-to-markdown --ignore-scripts
```

The renderer uses installed Microsoft Edge by default. To use bundled Chromium
instead, run `python -m playwright install chromium` and pass `--channel ""`
to `book_review.py`.

Prepare the selected PDF pages through Stage 08 and check that Stages 02, 06, 07
and 08 succeeded. Then start a fresh review:

```powershell
$Book = "tools/bootstrap/books/test-book-example-4567/78e52f075e17"
python tools/bootstrap/book-to-markdown/book_review.py import --book $Book
python tools/bootstrap/book-to-markdown/book_review.py render --book $Book
./tools/bootstrap/book-to-markdown/review-server.ps1 start -Book $Book
```

**Import deletes and rebuilds Working, Accepted and the copied PDF pages.** It
discards existing review edits and settings. Do not rerun it to continue a
review. The first render initializes Accepted from the new Working book.
`serve` also renders when needed, so the separate `render` command is optional
when starting the server. Use only one renderer or server for a book at a time.

For a foreground server, run
`python tools/bootstrap/book-to-markdown/book_review.py serve --book $Book`
and stop it with Ctrl+C. `--port` changes its port; `--width` changes the
preparation browser viewport, not the displayed document width.

### Continuing a review

Edit Working in the browser. The PDF pane shows the original prepared pages;
Accepted is the comparison baseline. **Accept Changes** publishes the completed
Working revision to Accepted and the variant-root `book.json`.
**Discard Changes** restores the complete Working bundle from Accepted; an element's
**Discard** restores only that element's properties. Neither action commits or
uploads files.

Use **Book Actions → Clean direct book text** to preview possible PDF references
for plain book text. Confirm each proposed occurrence against the PDF before
applying it. **Show source differences** marks source-backed corrections in
yellow, accepted text without a source address in blue, and pending plain text
in purple. These colors are review aids, not proof of transcription quality.

Use **Book Actions → Rename Chapters** to edit Working chapter titles and Markdown
filenames together. The generated table of contents has its own entry. Chapter
titles remain separate from filenames: edited chapter titles are aligned to PDF
text blocks where possible. **Clean direct book text** checks remaining direct
title text, including an edited contents title.
Filenames must be unique bundle-local `.md` names. Saving a rename updates
Working Markdown exports and contents links; the old generated filenames are
removed. Accept Changes publishes the renamed recipe and files.

Refresh a complete Working Markdown export after the current revision renders:

```powershell
python tools/bootstrap/book-to-markdown/book_review.py export --book $Book
```

## Book authoring sequence

1. Prepare the source PDF through [Stage 08](../pdf-to-markdown/stages/08_proofreading/README.md).
2. Import once, then arrange and correct chapters and elements in Working.
3. Run [source validation](#source-validation) and inspect source differences
   against the PDF. A valid address can still point to the wrong repeated text.
4. Review page corrections, chapter formatting, tables, graphics and descriptions.
   The [table](prompts/describe-tables.md) and
   [image](prompts/describe-images.md) prompts support requested authoring.
5. Review the rendered book and exports, then accept the intended revision.
   Standalone reconstruction and RAG ingestion remain in the
   [roadmap](../../../ROADMAP.md#step-1-manual-conversion-and-initial-knowledge-base).

## Source catalog and variant layout

Local source variants live under `tools/bootstrap/books/`:

```text
tools/bootstrap/books/<book-id>/<first-12-characters-of-sha256>/
  source.pdf          # local source or assembled composite PDF
  source.json         # tracked full SHA-256
  pdf-to-markdown/    # local preparation workspace
```

The current composite Test Book variant is
`test-book-example-4567/78e52f075e17`. Its
[page tracker](../books/test-book-example-4567/78e52f075e17/test_pages_tracker.md)
records the source variants and 160-page assembly order. The other source
variants are identified by their own `source.json` files. Source PDFs and
generated outputs remain local; the metadata is tracked.

When acquiring a source PDF, compare its **full** SHA-256 with the variant's
`source.json`, copy the matching file to `source.pdf`, and verify the copied
file's hash. For example:

```powershell
Get-FileHash -LiteralPath './downloaded.pdf' -Algorithm SHA256
```

A different hash requires the matching source or a separate variant; changing
an existing variant's hash does not make a different PDF equivalent.

## Editing and publication

`working/book.json` is the authored review model. The variant-root
`book.json` is the accepted recipe intended for later publication; generated
Markdown, renders and assets are replaceable local outputs. The PDF supplies
source text and graphics. A recipe records chapter order, element roles and
visibility, Markdown structure, source regions and explicit corrections.

Source-derived `md_text`, chapter titles and filenames use Stage 02 physical
page, block and Unicode character ranges. For example,
`{{text page=3 block=1 chars=0:6}}` selects characters 0 through 5.
`{{accepted text="Vision"}}` records visible text without a PDF address.
Formatting around references remains literal Markdown or HTML.
`resolved_md_text` is derived display text, not authored recipe content.
The [element vocabulary](elements.md) lists supported types and roles.

Import aligns Stage 08 text with Stage 02 PDF text, but OCR and repeated
phrases can produce uncertain or incorrect source attribution. The reviewer
must check the actual PDF. The recipe stores reviewed decisions so later
reconstruction can avoid repeating model inference; the standalone command
for reconstruction from a matching PDF is still pending.

### Source validation

```powershell
python tools/bootstrap/book-to-markdown/book_review.py validate --book $Book
```

This read-only command checks Working recipe references and reports visible
literal content without source addresses. It prints JSON and exits with code 1
when the audit fails. It does not change the book or prove that a valid range
identifies the correct printed occurrence.

## Codex Working task

Use **Book Actions → Codex...** for tasks across Working, or the context menu
to describe one selected image or table. The suggested tasks and description
rules live in [`prompts/`](prompts/); the authored `description` field and its
rendering are defined in [elements.md](elements.md#notes-legends-and-groups).

Codex edits Working at the current revision; Accepted and PDF sources remain
read-only. One-element transformations preserve source content by default;
their rules are in the [transformation prompt](prompts/codex-edit.md). Review
changes before acceptance. **Cancel task**, Escape, and
**Stop task** interrupt the active request; edits already saved to Working
remain. The model and effort are selected in [config.yaml](config.yaml).

Task diagnostics are under `.agent/tmp/book-codex-agent/<request-id>/`, and
element-transform diagnostics under `.agent/tmp/book-codex/<request-id>/`.
Shared authentication and cache behavior are in the
[conversion client README](../conversion/README.md).

## Tests

```powershell
python tools/verification/verify.py test python directory tools/bootstrap/book-to-markdown/tests
```

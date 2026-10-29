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

### Reconstructing a cloned book

After restoring the variant's matching `source.pdf`, use the accepted recipe at
the variant root to rebuild local editor inputs:

```powershell
$Book = "tools/bootstrap/books/test-book-example-4567/78e52f075e17"
python tools/bootstrap/book-to-markdown/book_review.py import-book --book $Book
python tools/bootstrap/book-to-markdown/book_review.py render --book $Book
./tools/bootstrap/book-to-markdown/review-server.ps1 start -Book $Book
```

`import-book` runs PDF Stages 01 and 02 for the full source, generating the OCR
PDF, positioned text and page PNGs. It copies the page PNGs into the editor,
extracts image crops from `book.json` PDF regions, copies the accepted recipe to
`working/book.json`, and exports Markdown. It does not run model conversion,
proofreading or the draft-generating `import` command. The first render creates
Accepted from the restored Working recipe; the root `book.json` stays intact.

The command checks `source.json` when present and compares a tracked
`source-ocr.json` with the OCR profile and processed page selection. OCR PDF
bytes may differ across preparation runs; `import-book` reports that difference
and validates that the saved text ranges still resolve. A matching source PDF
and stable positioned text are required for reliable source references.
It refuses to replace an existing Working or Accepted review. Existing recipes
with edited image assets beyond a single PDF crop need those assets or an
explicit image transformation recipe; `import-book` reports that case instead
of silently substituting another image. Run [source validation](#source-validation)
and inspect the PDF after reconstruction; validation cannot prove that a range
points to the intended printed occurrence.

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

After accepting the final revision, copy its Markdown chapters and referenced
images into `Obsidian/Amiga/Reference/<book-directory>/`:

```powershell
python tools/bootstrap/book-to-markdown/book_review.py publish --book $Book
```

The directory name comes from the book directory above the variant. Publication
uses the files already in Accepted, converts generated block links to ordinary
Markdown links to chapter headings, and omits review-only element anchors and
Obsidian block IDs from the published copy. Working and Accepted stay intact.
Repeated heading text receives a numeric suffix in the published chapter and
contents entry so every link names one heading. Publication also writes each
visible image's `description` from the accepted root `book.json` to
`assets/<image-name>.txt`. Images without a description have no sidecar;
hidden images are omitted. The command requires the root and Accepted recipes
to match, leaves review files intact, and refuses to replace an existing
Reference directory.
The published contents use relative Markdown links that can be navigated in
Obsidian and a standard Markdown preview. The review exports still use Obsidian
block links so identical headings retain distinct destinations during editing.
Before a commit, review each published Markdown file and add the `book`,
`chapter`, and content-based tags described in
[Reference Book Metadata](../../../.agents/rules/docs-maintenance.md#reference-book-metadata).
The commit check reports missing structural metadata; an agent asks before
editing frontmatter.

## Book authoring sequence

1. Prepare the source PDF through [Stage 08](../pdf-to-markdown/stages/08_proofreading/README.md).
2. Import once, then arrange and correct chapters and elements in Working.
3. Run [source validation](#source-validation) and inspect source differences
   against the PDF. A valid address can still point to the wrong repeated text.
4. Review page corrections, chapter organization, formatting, tables, graphics,
   and PDF source selections.
5. Review the rendered book and exports, accept the intended revision, and
   commit its `book.json` with the matching `source.json`. Preparing the four
   manual recipes remains in the
   [roadmap](../../../ROADMAP.md#step-11-four-manual-pdf-to-book-json-conversion).

The workflow below lets someone with a matching PDF continue from a committed
recipe, add descriptions, publish Markdown, and index it for RAG.

## Documentation bootstrap from an accepted `book.json`

Repeat this sequence for each reviewed book. The variant needs its accepted
`book.json`, matching `source.pdf`, and `source.json`; check the full source
SHA-256 as described in [source catalog](#source-catalog-and-variant-layout).
The RAG CLI and shared index state file are configured in the
[developer setup](../../../.agents/references/developer-setup.md#5-domain-hardware-knowledge-local-vector-rag-amiga-rag).

1. Run `book_review.py import-book --book <variant-directory>` to rebuild the
   editor from the accepted recipe and PDF. Then render and open the review
   server as in [reconstructing a cloned book](#reconstructing-a-cloned-book).
   `import-book` refuses an existing Working or Accepted bundle, so use it for
   a clean reconstruction, not to resume an active review.
2. Validate the reconstructed book and inspect its text, tables, graphics,
   chapter links, and PDF references. If image or table descriptions are still
   missing, use the Book Review Codex actions on Working, inspect the results
   against the PDF, and choose **Accept Changes**. This updates Accepted and
   the variant-root `book.json`; descriptions already in the recipe need no
   new model request. Review ASCII art in its source context.
3. Run `book_review.py publish --book <variant-directory>` after acceptance.
   It writes Markdown, images, and accepted image-description sidecars to
   `Obsidian/Amiga/Reference/<book-directory>/`. It refuses an occupied target;
   inspect the existing publication before any replacement. Review the
   published files and add the required [reference metadata](../../../.agents/rules/docs-maintenance.md#reference-book-metadata).
4. After validating the published Markdown, reindex the Amiga documentation
   scopes from the repository root. The CLI skips unchanged files:

   ```powershell
   $env:RAG_INDEX_JSON = "<shared-rag-index-state-file>"
   rag_qdrant docs --source amiga --index-json $env:RAG_INDEX_JSON
   rag_qdrant "Obsidian/Amiga/Design" --source amiga --index-json $env:RAG_INDEX_JSON
   rag_qdrant "Obsidian/Amiga/Reference" --source amiga --index-json $env:RAG_INDEX_JSON
   ```

5. Check the index status and search for distinctive chapter text, table
   descriptions, and image descriptions under source `amiga`. Compare returned
   fragments with the published Markdown and image `.txt` sidecars. The
   [RAG indexing workflow](../../../.agents/skills/index-amiga-rag/SKILL.md)
   has the status and search commands.

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
reconstruction can avoid repeating model inference through `import-book`.

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
